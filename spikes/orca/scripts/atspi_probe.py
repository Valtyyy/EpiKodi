#!/usr/bin/env python3
"""AT-SPI probe for the Orca spike.

1. Finds the "Orca spike" window in the AT-SPI desktop tree.
2. Dumps its accessible tree (role, name, states, attributes, Value/Text).
3. Listens for N seconds to focus, text-insert, children-changed and
   announcement events, and prints the ones coming from the spike app.
   Events from other applications (the terminal running this probe, etc.)
   are counted but not printed.
4. Deregisters its listeners, dumps the tree again and prints a per-type
   summary. The process then exits by itself.

The page drives its own focus changes (scripted timer in dist/index.html),
so no human input is needed. Usage:
    python3 -I scripts/atspi_probe.py [--seconds 35] [--max-depth 12]
"""
import argparse
import collections
import sys
import time

import gi

gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, GLib  # noqa: E402

WINDOW_TITLE = "Orca spike"
EVENTS = [
    "object:state-changed:focused",
    "object:text-changed:insert",
    "object:children-changed",
    "object:announcement",
]
T0 = time.monotonic()
EVENT_COUNTS = collections.Counter()
IGNORED_COUNTS = collections.Counter()
APP_NAME = None
ALL_MODE = False


def log(msg):
    print(f"[{time.monotonic() - T0:7.3f}s] {msg}", flush=True)


def safe(fn, default=""):
    try:
        value = fn()
        return default if value is None else value
    except GLib.Error as exc:  # D-Bus / AT-SPI errors on stale objects
        return f"<error: {exc.message}>"


def states_of(acc):
    state_set = acc.get_state_set()
    if state_set is None:
        return []
    return sorted(s.value_nick for s in state_set.get_states())


def extras_of(acc):
    """Attributes (aria-live, xml-roles, tag...) and Value/Text contents when exposed."""
    parts = []
    attrs = safe(acc.get_attributes, None)
    if isinstance(attrs, dict) and attrs:
        shown = {k: v for k, v in attrs.items()}
        parts.append(f"attrs={shown}")
    ifaces = safe(acc.get_interfaces, [])
    if isinstance(ifaces, list):
        if "Value" in ifaces:
            parts.append(f"value={safe(lambda: Atspi.Value.get_current_value(acc))}")
        if "Text" in ifaces:
            parts.append(f"text={safe(lambda: Atspi.Text.get_text(acc, 0, -1))!r}")
    return " ".join(parts)


def find_window(timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        desktop = Atspi.get_desktop(0)
        for i in range(desktop.get_child_count()):
            app = desktop.get_child_at_index(i)
            if app is None:
                continue
            app_name = safe(app.get_name)
            for j in range(app.get_child_count()):
                win = app.get_child_at_index(j)
                if win is not None and safe(win.get_name) == WINDOW_TITLE:
                    return app, app_name, win
        time.sleep(0.5)
    return None


def dump(acc, depth=0, max_depth=12):
    role = safe(acc.get_role_name)
    name = safe(acc.get_name)
    extra = extras_of(acc)
    log(f"{'  ' * depth}- role={role!r} name={name!r} states={states_of(acc)}" + (f" {extra}" if extra else ""))
    if depth >= max_depth:
        return
    for k in range(safe(acc.get_child_count, 0)):
        child = acc.get_child_at_index(k)
        if child is not None:
            dump(child, depth + 1, max_depth)


def belongs_to_spike(src):
    """True when the event source lives in the spike application."""
    if src is None:
        return False
    return safe(lambda: src.get_application().get_name()) == APP_NAME


def describe_event(event):
    src = event.source
    role = safe(src.get_role_name) if src else "?"
    name = safe(src.get_name) if src else "?"
    text = ""
    if event.type.startswith("object:announcement"):
        text = f" any_data={safe(lambda: event.any_data, None)!r}"
    return (f"type={event.type} source.role={role!r} source.name={name!r} "
            f"detail1={event.detail1} detail2={event.detail2}{text}")


def is_live_region(src):
    return safe(lambda: src.get_name()).startswith("Région")


def on_event(event):
    if not belongs_to_spike(event.source):
        IGNORED_COUNTS[event.type] += 1
        return
    EVENT_COUNTS[event.type] += 1
    if ALL_MODE:
        # Whole 'object' family: print only what touches a live region.
        if is_live_region(event.source) or "live" in safe(lambda: event.source.get_attributes(), {}):
            log(f"LIVE {describe_event(event)}")
        return
    log(f"EVENT {describe_event(event)}")


def main():
    global APP_NAME, ALL_MODE
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--seconds", type=float, default=35.0, help="listening duration")
    parser.add_argument("--max-depth", type=int, default=12, help="tree dump depth")
    parser.add_argument("--find-timeout", type=float, default=30.0, help="wait for the window to appear")
    parser.add_argument("--all-object-events", action="store_true",
                        help="register the whole 'object' event family and count every type "
                             "(prints only events whose source is a live region)")
    args = parser.parse_args()

    rc = Atspi.init()
    log(f"Atspi.init() -> {rc} (0 = ok)")

    found = find_window(args.find_timeout)
    if found is None:
        log(f"FAILED: no window named {WINDOW_TITLE!r} in the AT-SPI desktop tree")
        log("Desktop applications seen:")
        desktop = Atspi.get_desktop(0)
        for i in range(desktop.get_child_count()):
            app = desktop.get_child_at_index(i)
            if app is not None:
                log(f"  app name={safe(app.get_name)!r} windows={safe(app.get_child_count, 0)}")
        Atspi.exit()
        return 1

    app, app_name, window = found
    APP_NAME = app_name
    ALL_MODE = args.all_object_events
    log(f"FOUND app={app_name!r} window={safe(window.get_name)!r} role={safe(window.get_role_name)!r}")

    log("--- tree dump (start) ---")
    dump(window, max_depth=args.max_depth)

    listener = Atspi.EventListener.new(on_event)
    registered = []
    events_to_register = ["object"] if args.all_object_events else EVENTS
    for event_name in events_to_register:
        ok = Atspi.EventListener.register(listener, event_name)
        registered.append(event_name)
        log(f"register {event_name!r} -> {ok}")

    loop = GLib.MainLoop()

    def finish_dump():
        log("--- tree dump (end) ---")
        dump(window, max_depth=args.max_depth)
        return False

    def stop():
        log(f"listening window of {args.seconds:.0f}s elapsed, stopping")
        loop.quit()
        return False

    # One-shot timers: GLib removes them after they fire (they return False).
    GLib.timeout_add(int(args.seconds * 1000 * 0.5), finish_dump)
    GLib.timeout_add(int(args.seconds * 1000), stop)
    log(f"listening for {args.seconds:.0f}s ...")
    loop.run()

    for event_name in registered:
        ok = Atspi.EventListener.deregister(listener, event_name)
        log(f"deregister {event_name!r} -> {ok}")
    Atspi.exit()

    log("--- summary (events from the spike app) ---")
    if ALL_MODE:
        for event_type, count in sorted(EVENT_COUNTS.items()):
            log(f"  {event_type}: {count}")
    else:
        for event_name in EVENTS:
            log(f"  {event_name}: {EVENT_COUNTS.get(event_name, 0)}")
    log(f"  events from other applications (ignored): {sum(IGNORED_COUNTS.values())}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
