import { NavLink, Outlet, useNavigate } from "react-router-dom";

import { Button } from "@/components/ui/button";
import { useSession } from "@/stores/session";

const links = [
  { to: "/", label: "Табло", end: true },
  { to: "/accounts", label: "Сметкоплан", end: false },
  { to: "/counterparts", label: "Контрагенти", end: false },
  { to: "/products", label: "Стоки", end: false },
  { to: "/invoices", label: "Фактури", end: false },
  { to: "/settings", label: "Настройки", end: false },
  { to: "/vat", label: "ДДС", end: false },
  { to: "/closing", label: "Приключване", end: false },
  { to: "/banks", label: "Банки", end: false },
];

export default function AppShell() {
  const logout = useSession((s) => s.logout);
  const navigate = useNavigate();

  async function doLogout() {
    await logout();
    navigate("/login", { replace: true });
  }

  return (
    <div className="flex min-h-screen">
      <aside className="flex w-48 flex-col gap-1 border-r p-4">
        <span className="px-2 pb-3 text-lg font-semibold">dzambo</span>
        {links.map((l) => (
          <NavLink
            key={l.to}
            to={l.to}
            end={l.end}
            className={({ isActive }) =>
              `rounded-md px-3 py-2 text-sm ${
                isActive
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-muted"
              }`
            }
          >
            {l.label}
          </NavLink>
        ))}
      </aside>
      <div className="min-w-0 flex-1">
        <header className="flex justify-end border-b p-3">
          <Button variant="outline" onClick={doLogout}>
            Изход
          </Button>
        </header>
        <Outlet />
      </div>
    </div>
  );
}
