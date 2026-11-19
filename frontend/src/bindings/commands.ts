// Генерира се от tauri-specta при пускане на desktop app-а (debug build).
// Заглушка до първия export — не редактирай ръчно.
import { invoke } from "@tauri-apps/api/core";

export const commands = {
  async probe(): Promise<string> {
    return await invoke("probe");
  },
};