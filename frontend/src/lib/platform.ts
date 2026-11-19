// Адаптер между споделения UI и хоста. Web и desktop имплементации имат
// еднакъв интерфейс — споделените екрани не викат Tauri директно.

export interface Platform {
  readonly name: "web" | "desktop";
  getToken(): Promise<string | null>;
  setToken(token: string | null): Promise<void>;
  /** Печат: web отваря принт прозорец, desktop вика native командата. */
  printHtml(html: string): Promise<void>;
  /** Запис на файл: web сваля, desktop показва native диалог. */
  saveFile(filename: string, blob: Blob): Promise<void>;
}

export function isDesktop(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

const TOKEN_KEY = "dzambo.token";

export const webPlatform: Platform = {
  name: "web",
  async getToken() {
    return localStorage.getItem(TOKEN_KEY);
  },
  async setToken(token) {
    if (token) localStorage.setItem(TOKEN_KEY, token);
    else localStorage.removeItem(TOKEN_KEY);
  },
  async printHtml(html) {
    const win = window.open("", "_blank", "width=800,height=600");
    if (!win) throw new Error("Изскачащият прозорец е блокиран");
    win.document.write(html);
    win.document.close();
    win.focus();
    win.print();
  },
  async saveFile(filename, blob) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  },
};

export let platform: Platform = isDesktop() ? ({} as Platform) : webPlatform;

/** Desktop имплементацията се регистрира от Tauri обвивката (phase 0, стъпка 4). */
export function setPlatform(next: Platform): void {
  platform = next;
}
