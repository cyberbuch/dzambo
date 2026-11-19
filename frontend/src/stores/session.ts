import { create } from "zustand";

import { setBaseUrl, setTokenGetter } from "@/lib/api";
import { platform } from "@/lib/platform";

interface SessionState {
  token: string | null;
  serverUrl: string;
  activeCompanyId: number | null;
  hydrated: boolean;
  hydrate: () => Promise<void>;
  login: (token: string) => Promise<void>;
  logout: () => Promise<void>;
  setServerUrl: (url: string) => void;
  setActiveCompany: (id: number | null) => void;
}

const SERVER_URL_KEY = "dzambo.serverUrl";
const COMPANY_KEY = "dzambo.activeCompany";

export const DEFAULT_SERVER_URL = "http://127.0.0.1:8095";

export const useSession = create<SessionState>((set) => ({
  token: null,
  serverUrl: "",
  activeCompanyId: null,
  hydrated: false,
  hydrate: async () => {
    const storedUrl =
      localStorage.getItem(SERVER_URL_KEY) ?? (isSecureContext ? "" : DEFAULT_SERVER_URL);
    setBaseUrl(storedUrl);
    const token = await platform.getToken();
    const company = Number(localStorage.getItem(COMPANY_KEY)) || null;
    set({ token, serverUrl: storedUrl, activeCompanyId: company, hydrated: true });
  },
  login: async (token) => {
    await platform.setToken(token);
    set({ token });
  },
  logout: async () => {
    await platform.setToken(null);
    set({ token: null });
  },
  setServerUrl: (url) => {
    localStorage.setItem(SERVER_URL_KEY, url);
    setBaseUrl(url);
    set({ serverUrl: url });
  },
  setActiveCompany: (id) => {
    if (id) localStorage.setItem(COMPANY_KEY, String(id));
    else localStorage.removeItem(COMPANY_KEY);
    set({ activeCompanyId: id });
  },
}));

setTokenGetter(() => useSession.getState().token);
