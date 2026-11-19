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
import type { AccountDto } from "@/bindings/AccountDto";
import type { ClosingRowDto } from "@/bindings/ClosingRowDto";
import type { OpeningDto } from "@/bindings/OpeningDto";
import type { Paged } from "@/bindings/Paged";

export default function ClosingPage() {
  const qc = useQueryClient();
  const year = new Date().getFullYear();
  const [period, setPeriod] = useState(() => new Date().toISOString().slice(0, 7));
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const accounts = useQuery({
    queryKey: ["accounts"],
    queryFn: () => api.get<Paged<AccountDto>>("/v1/accounts?limit=500"),
  });
  const opening = useQuery({
    queryKey: ["opening", year],
    queryFn: () =>
      api.get<{ items: OpeningDto[] }>(`/v1/opening-balances?year=${year}`),
  });
  const preview = useQuery({
    queryKey: ["closing-preview", period],
    queryFn: () =>
      api.get<ClosingRowDto[]>(
        `/v1/closing-operations/preview?year=${period.slice(0, 4)}&month=${Number(period.slice(5, 7))}`,
      ),
  });

  const [accId, setAccId] = useState("");
  const [debit, setDebit] = useState("0");
  const [credit, setCredit] = useState("0");

  const saveOpening = useMutation({
    mutationFn: () =>
      api.put("/v1/opening-balances", {
        account_id: Number(accId),
        year,
        debit,
        credit,
      }),
    onSuccess: () => {
      setMessage("салдото е записано");
      void qc.invalidateQueries({ queryKey: ["opening"] });
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const runClosing = useMutation({
    mutationFn: () =>
      api.post("/v1/closing-operations/run", {
        year: Number(period.slice(0, 4)),
        month: Number(period.slice(5, 7)),
      }),
    onSuccess: () => {
      setMessage("приключването е записано");
      void qc.invalidateQueries({ queryKey: ["closing-preview"] });
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const acctName = (id: number) =>
    accounts.data?.items.find((a) => a.id === id)?.number ?? String(id);

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Салда и приключване</h1>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {message ? <p className="text-sm text-muted-foreground">{message}</p> : null}

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            Начални салда ({year})
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="mb-4 flex flex-wrap items-end gap-3">
            <div className="flex min-w-52 flex-col gap-1">
              <Label>Сметка</Label>
              <select
                value={accId}
                onChange={(e) => setAccId(e.target.value)}
                className="h-9 rounded-md border bg-background px-2 text-sm"
              >
                <option value="">—</option>
                {(accounts.data?.items ?? []).map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.number} {a.name}
                  </option>
                ))}
              </select>
            </div>
            <div className="flex w-28 flex-col gap-1">
              <Label>Дебит</Label>
              <Input value={debit} onChange={(e) => setDebit(e.target.value)} />
            </div>
            <div className="flex w-28 flex-col gap-1">
              <Label>Кредит</Label>
              <Input value={credit} onChange={(e) => setCredit(e.target.value)} />
            </div>
            <Button
              disabled={!accId || saveOpening.isPending}
              onClick={() => saveOpening.mutate()}
            >
              Запиши
            </Button>
          </div>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Сметка</TableHead>
                <TableHead className="text-right">Дебит</TableHead>
                <TableHead className="text-right">Кредит</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(opening.data?.items ?? []).map((o) => (
                <TableRow key={o.id}>
                  <TableCell>{acctName(o.account_id)}</TableCell>
                  <TableCell className="text-right">{o.debit}</TableCell>
                  <TableCell className="text-right">{o.credit}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            Приключвателни операции
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="mb-4 flex flex-wrap items-end gap-3">
            <div className="flex flex-col gap-1">
              <Label>Месец</Label>
              <Input
                type="month"
                value={period}
                onChange={(e) => setPeriod(e.target.value)}
                className="w-36"
              />
            </div>
            <Button
              variant="outline"
              onClick={() => void preview.refetch()}
            >
              Преглед
            </Button>
            <Button
              variant="destructive"
              disabled={runClosing.isPending || !(preview.data?.length)}
              onClick={() => runClosing.mutate()}
            >
              Приключи месеца
            </Button>
          </div>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Източник</TableHead>
                <TableHead>Цел</TableHead>
                <TableHead className="text-right">Салдо</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(preview.data ?? []).map((r) => (
                <TableRow key={r.source}>
                  <TableCell>
                    {r.source} {r.source_name}
                  </TableCell>
                  <TableCell>{r.target}</TableCell>
                  <TableCell className="text-right">{r.amount}</TableCell>
                </TableRow>
              ))}
              {!(preview.data?.length) ? (
                <TableRow>
                  <TableCell colSpan={3} className="text-muted-foreground">
                    Няма салда за приключване.
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
