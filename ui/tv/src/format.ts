import type { AppError, PingReply } from "@om/shared";

export function formatPing(reply: PingReply): string {
  return `${reply.message} (${reply.timestamp_ms})`;
}

export function formatError(err: AppError): string {
  return err.suggestion ? `${err.message}. ${err.suggestion}` : err.message;
}
