import { useEffect, useState } from "react";

import { api, ApiError } from "@/lib/api";
import { isDesktop, platform } from "@/lib/platform";
import { useSession } from "@/stores/session";

interface Health {
  status: string;
}

export default function App() {
  const hydrated = useSession((s) => s.hydrated);
  const hydrate = useSession((s) => s.hydrate);
  const [health, setHealth] = useState<string>("—");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  async function ping() {
    try {
      const data = await api.get<Health>("/health");
      setHealth(data.status);
      setError(null);
    } catch (e) {
      setHealth("off");
      setError(e instanceof ApiError ? e.message : String(e));
    }
  }

  if (!hydrated) return <div className="p-6">Зареждане…</div>;

  return (
    <main className="mx-auto flex min-h-screen max-w-md flex-col gap-4 p-6">
      <h1 className="text-2xl font-semibold">dzambo</h1>
      <p className="text-sm text-muted-foreground">
        хост: {isDesktop() ? "desktop" : "web"} · сървър:{" "}
        {useSession.getState().serverUrl || "—"}
      </p>
      <button
        onClick={ping}
        className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
      >
        Провери /health
      </button>
      <p className="text-sm">
        състояние: {health}
        {error ? ` · ${error}` : ""}
      </p>
      <span className="text-xs text-muted-foreground">platform: {platform.name}</span>
    </main>
  );
}
