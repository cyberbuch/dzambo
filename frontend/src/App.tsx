import { useEffect } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";

import DashboardPage from "@/features/dashboard/DashboardPage";
import LoginPage from "@/features/auth/LoginPage";
import { useSession } from "@/stores/session";

export default function App() {
  const hydrated = useSession((s) => s.hydrated);
  const token = useSession((s) => s.token);
  const hydrate = useSession((s) => s.hydrate);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  if (!hydrated) return <div className="p-6">Зареждане…</div>;

  return (
    <BrowserRouter>
      <Routes>
        <Route
          path="/login"
          element={token ? <Navigate to="/" replace /> : <LoginPage />}
        />
        <Route
          path="/"
          element={token ? <DashboardPage /> : <Navigate to="/login" replace />}
        />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
