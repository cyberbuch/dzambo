import { useEffect } from "react";
import { BrowserRouter, Navigate, Outlet, Route, Routes } from "react-router-dom";

import AppShell from "@/components/AppShell";
import AccountsPage from "@/features/accounts/AccountsPage";
import CounterpartsPage from "@/features/counterparts/CounterpartsPage";
import DashboardPage from "@/features/dashboard/DashboardPage";
import InvoicesPage from "@/features/invoices/InvoicesPage";
import LoginPage from "@/features/auth/LoginPage";
import ProductsPage from "@/features/products/ProductsPage";
import SettingsPage from "@/features/settings/SettingsPage";
import { useSession } from "@/stores/session";

function Guard() {
  const token = useSession((s) => s.token);
  if (!token) return <Navigate to="/login" replace />;
  return <Outlet />;
}

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
        <Route element={<Guard />}>
          <Route element={<AppShell />}>
            <Route index element={<DashboardPage />} />
            <Route path="accounts" element={<AccountsPage />} />
            <Route path="counterparts" element={<CounterpartsPage />} />
            <Route path="products" element={<ProductsPage />} />
            <Route path="invoices" element={<InvoicesPage />} />
            <Route path="settings" element={<SettingsPage />} />
          </Route>
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
