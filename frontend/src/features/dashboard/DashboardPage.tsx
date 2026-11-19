import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import { useSession } from "@/stores/session";
import type { CompanyDto } from "@/bindings/CompanyDto";
import type { DashboardDto } from "@/bindings/DashboardDto";
import type { MeDto } from "@/bindings/MeDto";

function Stat({ label, value }: { label: string; value: string | number }) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-xs font-medium text-muted-foreground">{label}</CardTitle>
      </CardHeader>
      <CardContent className="text-xl font-semibold">{value}</CardContent>
    </Card>
  );
}

export default function DashboardPage() {
  const logout = useSession((s) => s.logout);
  const navigate = useNavigate();

  const me = useQuery({
    queryKey: ["me"],
    queryFn: () => api.get<MeDto>("/v1/me"),
  });
  const company = useQuery({
    queryKey: ["active-company"],
    queryFn: async () => {
      const res = await api.get<CompanyDto | Record<string, never>>("/v1/active-company");
      return "id" in res ? (res as CompanyDto) : null;
    },
  });
  const dash = useQuery({
    queryKey: ["dashboard"],
    queryFn: () => api.get<DashboardDto>("/v1/dashboard"),
  });

  async function doLogout() {
    await logout();
    navigate("/login", { replace: true });
  }

  if (me.isLoading || dash.isLoading) {
    return <div className="p-6">Зареждане…</div>;
  }
  if (me.error || dash.error) {
    return (
      <div className="p-6 text-destructive">
        Грешка при зареждане: {String(me.error ?? dash.error)}
      </div>
    );
  }

  const d = dash.data!;
  const active = company.data?.name ?? "—";
  const user = me.data!;

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Табло</h1>
          <p className="text-sm text-muted-foreground">
            {active} · {user.name}
          </p>
        </div>
        <Button variant="outline" onClick={doLogout}>
          Изход
        </Button>
      </header>

      <section className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <Stat label="Чернови фактури" value={d.invoices_draft} />
        <Stat label="Постнати фактури" value={d.invoices_posted} />
        <Stat label="Вземания" value={d.receivables} />
        <Stat label="Задължения" value={d.payables} />
        <Stat label="ДДС начислен" value={d.vat_output} />
        <Stat label="ДДС по признаване" value={d.vat_input} />
        <Stat label="Приходи" value={d.revenue_ytd} />
        <Stat label="Разходи" value={d.expense_ytd} />
      </section>

      <Card>
        <CardHeader>
          <CardTitle>Последни фактури</CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Номер</TableHead>
                <TableHead>Дата</TableHead>
                <TableHead>Контрагент</TableHead>
                <TableHead>Валута</TableHead>
                <TableHead className="text-right">Сума</TableHead>
                <TableHead>Статут</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {d.recent.map((r) => (
                <TableRow key={r.id}>
                  <TableCell>{r.number}</TableCell>
                  <TableCell>{r.issue_date}</TableCell>
                  <TableCell>{r.counterpart_name}</TableCell>
                  <TableCell>{r.currency}</TableCell>
                  <TableCell className="text-right">{r.total_amount}</TableCell>
                  <TableCell>{r.status}</TableCell>
                </TableRow>
              ))}
              {d.recent.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} className="text-muted-foreground">
                    Няма фактури.
                  </TableCell>
                </TableRow>
              ) : null}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </main>
  );
}
