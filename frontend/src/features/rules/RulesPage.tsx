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
import type { BankRuleDto } from "@/bindings/BankRuleDto";
import type { Paged } from "@/bindings/Paged";

export default function RulesPage() {
  const qc = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [accountId, setAccountId] = useState("");
  const [name, setName] = useState("");
  const [keywords, setKeywords] = useState("");
  const [direction, setDirection] = useState("any");
  const [priority, setPriority] = useState("0");
  const [accountRuleId, setAccountRuleId] = useState("");

  const accounts = useQuery({
    queryKey: ["accounts"],
    queryFn: () => api.get<Paged<AccountDto>>("/v1/accounts?limit=500"),
  });
  const rules = useQuery({
    queryKey: ["bank-rules"],
    queryFn: () => api.get<BankRuleDto[]>("/v1/bank-matching-rules"),
  });

  const refresh = () => {
    void qc.invalidateQueries({ queryKey: ["bank-rules"] });
    void qc.invalidateQueries({ queryKey: ["bank-txs"] });
  };

  const create = useMutation({
    mutationFn: () =>
      api.post("/v1/bank-matching-rules", {
        name,
        keywords,
        direction,
        priority: Number(priority) || 0,
        is_active: true,
        account_id: Number(accountRuleId),
      }),
    onSuccess: () => {
      setName("");
      setKeywords("");
      setMessage("правилото е добавено");
      refresh();
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const remove = useMutation({
    mutationFn: (id: number) => api.delete(`/v1/bank-matching-rules/${id}`),
    onSuccess: refresh,
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const defaults = useMutation({
    mutationFn: () => api.post<{ created: number }>("/v1/bank-matching-rules/defaults"),
    onSuccess: (d) => {
      setMessage(`създадени правила: ${d.created}`);
      refresh();
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const allocate = useMutation({
    mutationFn: () =>
      api.post<{ posted: number; failed: number }>(
        "/v1/bank-matching-rules/auto-allocate",
        { account_id: Number(accountId) },
      ),
    onSuccess: (d) => setMessage(`разнесени: ${d.posted}, пропуснати: ${d.failed}`),
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Банкови правила</h1>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {message ? <p className="text-sm text-muted-foreground">{message}</p> : null}

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">Ново правило</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="mb-4 flex flex-wrap items-end gap-3">
            <div className="flex min-w-48 flex-col gap-1">
              <Label>Име</Label>
              <Input value={name} onChange={(e) => setName(e.target.value)} />
            </div>
            <div className="flex min-w-64 flex-col gap-1">
              <Label>Ключови думи (със запетая)</Label>
              <Input value={keywords} onChange={(e) => setKeywords(e.target.value)} />
            </div>
            <div className="flex flex-col gap-1">
              <Label>Посока</Label>
              <select value={direction} onChange={(e) => setDirection(e.target.value)} className="h-9 rounded-md border bg-background px-2 text-sm">
                <option value="any">всяка</option>
                <option value="debit">разход (D)</option>
                <option value="credit">приход (C)</option>
              </select>
            </div>
            <div className="flex w-20 flex-col gap-1">
              <Label>Приоритет</Label>
              <Input value={priority} onChange={(e) => setPriority(e.target.value)} />
            </div>
            <div className="flex min-w-48 flex-col gap-1">
              <Label>Сметка</Label>
              <select value={accountRuleId} onChange={(e) => setAccountRuleId(e.target.value)} className="h-9 rounded-md border bg-background px-2 text-sm">
                <option value="">—</option>
                {(accounts.data?.items ?? []).map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.number} {a.name}
                  </option>
                ))}
              </select>
            </div>
            <Button disabled={!name || !keywords || create.isPending} onClick={() => create.mutate()}>
              Добави
            </Button>
          </div>
          <Button variant="outline" onClick={() => defaults.mutate()} disabled={defaults.isPending}>
            Зареди правила по подразбиране
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            Автоматично разнасяне
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap items-end gap-3">
            <div className="flex min-w-48 flex-col gap-1">
              <Label>Банкова сметка</Label>
              <select value={accountId} onChange={(e) => setAccountId(e.target.value)} className="h-9 rounded-md border bg-background px-2 text-sm">
                <option value="">—</option>
                {(accounts.data?.items ?? [])
                  .filter((a) => ["501", "502", "503"].includes(a.number.slice(0, 3)))
                  .map((a) => (
                    <option key={a.id} value={a.id}>
                      {a.number} {a.name}
                    </option>
                  ))}
              </select>
            </div>
            <Button disabled={!accountId || allocate.isPending} onClick={() => allocate.mutate()}>
              Разнеси по правилата
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-4">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Име</TableHead>
                <TableHead>Ключови думи</TableHead>
                <TableHead>Посока</TableHead>
                <TableHead>Приоритет</TableHead>
                <TableHead></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(rules.data ?? []).map((r) => (
                <TableRow key={r.id}>
                  <TableCell>{r.name}</TableCell>
                  <TableCell className="max-w-72 truncate">{r.keywords}</TableCell>
                  <TableCell>{r.direction}</TableCell>
                  <TableCell>{r.priority}</TableCell>
                  <TableCell>
                    <Button size="sm" variant="ghost" onClick={() => remove.mutate(r.id)}>
                      Изтрий
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </main>
  );
}
