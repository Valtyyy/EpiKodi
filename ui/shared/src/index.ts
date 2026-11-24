// Types generated from Rust by ts-rs (`pnpm gen:types`). Never hand-write them.
export type { AppError } from "./generated/AppError";
export type { PingReply } from "./generated/PingReply";
export type { ApplyMoment } from "./generated/ApplyMoment";
export type { SecretKey } from "./generated/SecretKey";
export type { SecretSource } from "./generated/SecretSource";
export type { SecretState } from "./generated/SecretState";
export type { SecretStatus } from "./generated/SecretStatus";
export type { SettingChange } from "./generated/SettingChange";
export type { SettingDescriptor } from "./generated/SettingDescriptor";
export type { SettingEntry } from "./generated/SettingEntry";
export type { SettingKind } from "./generated/SettingKind";
export type { SettingSource } from "./generated/SettingSource";
export type { SettingValue } from "./generated/SettingValue";
export type { SettingWarning } from "./generated/SettingWarning";
export type { SettingsPatch } from "./generated/SettingsPatch";
export type { SettingsSnapshot } from "./generated/SettingsSnapshot";

export {
  useRovingGrid,
  findNext,
  createSpatialNavigation,
  useSpatialNavigation,
  focusRing,
  NAV_ITEM_ATTR,
  NAV_ZONE_ATTR,
} from "./focus";
export type {
  GridItemProps,
  GridPosition,
  RovingGrid,
  UseRovingGridOptions,
  Direction,
  Rect,
  SpatialNavigation,
} from "./focus";

export { ApiProvider, useApi, createFakeApiClient } from "./api";
export type { ApiClient } from "./api";

export {
  applyTheme,
  parseTheme,
  contrastRatio,
  CONTRAST_PAIRS,
  MIN_CONTRAST,
  THEME_COLORS,
} from "./theme";
export type { Theme, ThemeColor, ThemeResult } from "./theme";
