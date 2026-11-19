import { useState } from "react";

import { useQuery } from "@tanstack/react-query";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api, ApiError } from "@/lib/api";
import type { VatRowDto } from "@/bindings/VatRowDto";

export default function VatPage() {
  const [period, setPeriod] = useState(() => new Date().toISOString().slice(0, 7));
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const sales = useQuery({
    queryKey: ["vat-sales", period],
    queryFn: () => api.get<VatRowDto[]>(`/v1/vat/sales?period=${period}`),
  });
  const purchases = useQuery({
    queryKey: ["vat-purchases", period],
    queryFn: () => api.get<VatRowDto[]>(`/v1/vat/purchases?period=${period}`),
  });
  const periods = useQuery({
    queryKey: ["periods"],
    queryFn: async () =>
      api.get<{ year: number; month: number; status: string }[]>(
        `/v1/accounting-periods?year=${period.slice(0, 4)}`,
      ),
  });

  const current = periods.data?.find(
    (p) => `${p.year}-${String(p.month).padStart(2, "0")}` === period,
  );
  const closed = current?.status === "closed";

  async function closeMonth(close: boolean) {
    try {
      setError(null);
      await api.post(`/v1/accounting-periods/${close ? "close" : "reopen"}`, {
        year: Number(period.slice(0, 4)),
        month: Number(period.slice(5, 7)),
      });
      setMessage(close ? "месецът е затворен" : "месецът е отворен");
      void periods.refetch();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    }
  }

  function renderTable(rows: VatRowDto[], title: string) {
    const vat = rows.reduce((s, r) => s + Number(r.vat_amount), 0);
    return (
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            {title} ({rows.length}) · ДДС: {vat.toFixed(2)}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Номер</TableHead>
                <TableHead>Дата</TableHead>
                <TableHead>Контрагент</TableHead>
                <TableHead>ЕИК</TableHead>
                <TableHead className="text-right">Нето</TableHead>
                <TableHead className="text-right">ДДС</TableHead>
                <TableHead className="text-right">Общо</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((r) => (
                <TableRow key={r.number}>
                  <TableCell>{r.number}</TableCell>
                  <TableCell>{r.issue_date}</TableCell>
                  <TableCell>{r.counterpart_name}</TableCell>
                  <TableCell>{r.eik}</TableCell>
                  <TableCell className="text-right">{r.net_amount}</TableCell>
                  <TableCell className="text-right">{r.vat_amount}</TableCell>
                  <TableCell className="text-right">{r.total_amount}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    );
  }

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">ДДС</h1>

      <div className="flex flex-wrap items-center gap-3">
        <div className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">Месец</span>
          <Input
            type="month"
            value={period}
            onChange={(e) => setPeriod(e.target.value)}
            className="w-36"
          />
        </div>
        <Button
          variant={closed ? "default" : "destructive"}
          disabled={!current}
          onClick={() => void closeMonth(!closed)}
        >
          {closed ? "Отвори месеца" : "Затвори месеца"}
        </Button>
        {error ? <span className="text-sm text-destructive">{error}</span> : null}
        {message ? <span className="text-sm text-muted-foreground">{message}</span> : null}
      </div>

      <div className="flex flex-wrap gap-2">
        <a
          href={`/v1/vat/file?period=${period}`}
          download
          className="inline-flex h-9 items-center rounded-md bg-secondary px-4 text-sm font-medium text-secondary-foreground hover:bg-secondary/80"
        >
          Свали CSV за НАП
        </a>
      </div>

      {renderTable(sales.data ?? [], "Продажби")}
      {renderTable(purchases.data ?? [], "Покупки")}
    </main>
  );
}
