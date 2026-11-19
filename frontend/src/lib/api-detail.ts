// Форматиране на грешки от API-то: {"detail": ...} като низ за потребителя.

export function formatApiDetail(data: unknown, fallback: string): string {
  if (typeof data === "string" && data.length > 0) return data;
  if (data === null || data === undefined || typeof data !== "object") return fallback;
  const detail = (data as { detail?: unknown }).detail;
  if (typeof detail === "string" && detail.length > 0) return detail;
  if (Array.isArray(detail)) {
    const parts: string[] = [];
    for (const item of detail) {
      if (typeof item === "string") {
        parts.push(item);
        continue;
      }
      if (item && typeof item === "object") {
        const o = item as { msg?: unknown; loc?: unknown };
        const msg = typeof o.msg === "string" ? o.msg : JSON.stringify(item);
        if (Array.isArray(o.loc) && o.loc.length > 0) {
          const loc = o.loc.filter((x): x is string => typeof x === "string").join(".");
          parts.push(loc ? `${loc}: ${msg}` : msg);
        } else {
          parts.push(msg);
        }
      }
    }
    if (parts.length > 0) return parts.join("; ");
  }
  if (detail !== undefined) {
    try {
      return JSON.stringify(detail);
    } catch {
      return fallback;
    }
  }
  return fallback;
}
