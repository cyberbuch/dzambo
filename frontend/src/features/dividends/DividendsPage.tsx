import { useState } from "react";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api, ApiError } from "@/lib/api";
import type { DividendShareDto } from "@/bindings/DividendShareDto";

export default function DividendsPage() {
  const qc = useQueryClient();
  const year = new Date().getFullYear();
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [formYear, setFormYear] = useState(String(year));
  const [total, setTotal] = useState("");
  const [decisionDate, setDecisionDate] = useState(() =>
    new Date().toISOString().slice(0, 10),
  );
  const [decisionNumber, setDecisionNumber] = useState("");

  const shares = useQuery({
    queryKey: ["dividends", formYear],
    queryFn: () => api.get<{ items: DividendShareDto[]; count: number; total_net: string }>(
      `/v1/dividends?year=${formYear}`,
    ),
  });

  const create = useMutation({
    mutationFn: () =>
      api.post<{ distribution_id: number; shares: number }>("/v1/dividends", {
        year: Number(formYear),
        total_amount: total,
        decision_date: decisionDate,
        decision_number: decisionNumber,
      }),
    onSuccess: (d) => {
      setMessage(`разпределение №${d.distribution_id}, дялове: ${d.shares}`);
      void qc.invalidateQueries({ queryKey: ["dividends"] });
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Дивиденти</h1>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {message ? <p className="text-sm text-muted-foreground">{message}</p> : null}

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">Разпределение</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap items-end gap-3">
            <div className="flex w-24 flex-col gap-1">
              <Label>Година</Label>
              <Input value={formYear} onChange={(e) => setFormYear(e.target.value)} />
            </div>
            <div className="flex w-36 flex-col gap-1">
              <Label>Обща сума</Label>
              <Input value={total} onChange={(e) => setTotal(e.target.value)} />
            </div>
            <div className="flex flex-col gap-1">
              <Label>Дата на решение</Label>
              <Input type="date" value={decisionDate} onChange={(e) => setDecisionDate(e.target.value)} />
            </div>
            <div className="flex flex-col gap-1">
              <Label>Номер на решение</Label>
              <Input value={decisionNumber} onChange={(e) => setDecisionNumber(e.target.value)} />
            </div>
            <Button disabled={!total || create.isPending} onClick={() => create.mutate()}>
              Създай
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            Дялове ({shares.data?.count ?? 0})
          </CardTitle>
          <span className="text-sm font-medium">Нето: {shares.data?.total_net ?? "—"}</span>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Собственик</TableHead>
                <TableHead>ЕГН</TableHead>
                <TableHead>Дял</TableHead>
                <TableHead className="text-right">Бруто</TableHead>
                <TableHead className="text-right">Данък</TableHead>
                <TableHead className="text-right">Нето</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(shares.data?.items ?? []).map((s) => (
                <TableRow key={s.id}>
                  <TableCell>{s.owner_name}</TableCell>
                  <TableCell>{s.owner_egn}</TableCell>
                  <TableCell>{s.ownership_percentage}</TableCell>
                  <TableCell className="text-right">{s.gross_amount}</TableCell>
                  <TableCell className="text-right">{s.tax_amount}</TableCell>
                  <TableCell className="text-right">{s.net_amount}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </main>
  );
}
