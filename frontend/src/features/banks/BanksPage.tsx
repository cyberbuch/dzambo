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
import type { BankAccountDto } from "@/bindings/BankAccountDto";
import type { BankTransactionDto } from "@/bindings/BankTransactionDto";

export default function BanksPage() {
  const qc = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [name, setName] = useState("");
  const [iban, setIban] = useState("");
  const [csv, setCsv] = useState("");

  const accounts = useQuery({
    queryKey: ["bank-accounts"],
    queryFn: () => api.get<BankAccountDto[]>("/v1/bank-accounts"),
  });
  const txs = useQuery({
    queryKey: ["bank-txs", selected],
    queryFn: () =>
      api.get<BankTransactionDto[]>(`/v1/bank-transactions?account_id=${selected}`),
    enabled: selected !== null,
  });

  const createAccount = useMutation({
    mutationFn: () =>
      api.post<BankAccountDto>("/v1/bank-accounts", { name, iban, currency: "EUR" }),
    onSuccess: (a) => {
      setName("");
      setIban("");
      setSelected(a.id);
      void qc.invalidateQueries({ queryKey: ["bank-accounts"] });
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const importCsv = useMutation({
    mutationFn: () =>
      api.post("/v1/bank-transactions/import", { account_id: selected, csv }),
    onSuccess: () => {
      setCsv("");
      void qc.invalidateQueries({ queryKey: ["bank-txs"] });
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const post = useMutation({
    mutationFn: (id: number) => api.post(`/v1/bank-transactions/${id}/post`),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ["bank-txs"] }),
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Банки</h1>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">Банкови сметки</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="mb-4 flex flex-wrap items-end gap-3">
            <div className="flex min-w-48 flex-col gap-1">
              <Label>Име</Label>
              <Input value={name} onChange={(e) => setName(e.target.value)} />
            </div>
            <div className="flex min-w-56 flex-col gap-1">
              <Label>IBAN</Label>
              <Input value={iban} onChange={(e) => setIban(e.target.value)} />
            </div>
            <Button disabled={!name || createAccount.isPending} onClick={() => createAccount.mutate()}>
              Добави
            </Button>
          </div>
          <div className="flex flex-wrap gap-2">
            {(accounts.data ?? []).map((a) => (
              <Button
                key={a.id}
                variant={selected === a.id ? "default" : "outline"}
                onClick={() => setSelected(a.id)}
              >
                {a.name}
              </Button>
            ))}
          </div>
        </CardContent>
      </Card>

      {selected !== null ? (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">Транзакции</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="mb-4 flex flex-col gap-2">
              <Label>Импорт — CSV, MT940 или camt.053 (авто-разпознаване)</Label>
              <textarea
                value={csv}
                onChange={(e) => setCsv(e.target.value)}
                rows={3}
                className="rounded-md border bg-background p-2 font-mono text-xs"
              />
              <Button
                className="w-fit"
                disabled={!csv || importCsv.isPending}
                onClick={() => importCsv.mutate()}
              >
                Импортирай
              </Button>
            </div>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Дата</TableHead>
                  <TableHead>Посока</TableHead>
                  <TableHead className="text-right">Сума</TableHead>
                  <TableHead>Контрагент</TableHead>
                  <TableHead>Статус</TableHead>
                  <TableHead></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(txs.data ?? []).map((t) => (
                  <TableRow key={t.id}>
                    <TableCell>{t.tx_date}</TableCell>
                    <TableCell>{t.direction}</TableCell>
                    <TableCell className="text-right">{t.amount}</TableCell>
                    <TableCell>{t.counterpart_name}</TableCell>
                    <TableCell>{t.status}</TableCell>
                    <TableCell>
                      {t.status !== "posted" ? (
                        <Button size="sm" onClick={() => post.mutate(t.id)}>
                          Разнеси
                        </Button>
                      ) : null}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      ) : null}
    </main>
  );
}
