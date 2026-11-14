"""Test double for scenario 4 (unresponsive Secret Service), run INSIDE an isolated bus.

Usage: python3 -I fake-secrets.py {silent|locked-prompt}

- silent: owns org.freedesktop.secrets but never answers OpenSession (first call of
  SecretService::connect). Models a keyring that never replies.
- locked-prompt: a collection that reports Locked=true; SearchItems returns one locked
  item; Unlock returns a prompt object whose Prompt method is accepted but never emits
  Completed. Models a keyring waiting for an unlock dialog that nobody answers.

Every received method call is logged to stdout. Not a real keyring: test double only.
"""
import sys
import time

import dbus
import dbus.mainloop.glib
import dbus.service
from gi.repository import GLib

MODE = sys.argv[1] if len(sys.argv) > 1 else "silent"
COLLECTION = "/org/freedesktop/secrets/collection/login"
ITEM = COLLECTION + "/1"
PROMPT = "/org/freedesktop/secrets/prompt/mock"
SERVICE_PATH = "/org/freedesktop/secrets"

dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
bus = dbus.SessionBus()
name = dbus.service.BusName("org.freedesktop.secrets", bus)


def log(*parts):
    print(MODE, *parts, flush=True)


class SecretServiceMock(dbus.service.Object):
    @dbus.service.method("org.freedesktop.Secret.Service", in_signature="sv", out_signature="vo")
    def OpenSession(self, algorithm, _input):
        log("OpenSession", algorithm)
        if MODE == "silent":
            log("never replying")
            time.sleep(600)
        return dbus.String(""), dbus.ObjectPath("/org/freedesktop/secrets/session/mock")

    @dbus.service.method("org.freedesktop.Secret.Service", in_signature="s", out_signature="o")
    def ReadAlias(self, alias):
        log("ReadAlias", alias)
        return dbus.ObjectPath(COLLECTION)

    @dbus.service.method("org.freedesktop.Secret.Service", in_signature="a{ss}", out_signature="aoao")
    def SearchItems(self, attributes):
        log("SearchItems", dict(attributes))
        return [], [dbus.ObjectPath(ITEM)]

    @dbus.service.method("org.freedesktop.Secret.Service", in_signature="ao", out_signature="aoo")
    def Unlock(self, objects):
        log("Unlock", [str(o) for o in objects], "-> returns prompt, nobody will answer it")
        return [], dbus.ObjectPath(PROMPT)


class CollectionMock(dbus.service.Object):
    @dbus.service.method("org.freedesktop.DBus.Properties", in_signature="ss", out_signature="v")
    def Get(self, interface, prop):
        log("Properties.Get", interface, prop)
        if prop == "Locked":
            return dbus.Boolean(True)
        return dbus.String("")


class PromptMock(dbus.service.Object):
    @dbus.service.method("org.freedesktop.Secret.Prompt", in_signature="s", out_signature="")
    def Prompt(self, window_id):
        log("Prompt called; no Completed signal will be sent")


class ItemMock(dbus.service.Object):
    pass


svc = SecretServiceMock(bus, SERVICE_PATH)
if MODE == "locked-prompt":
    CollectionMock(bus, COLLECTION)
    PromptMock(bus, PROMPT)
print(f"fake Secret Service ready, mode={MODE}", flush=True)
GLib.MainLoop().run()
