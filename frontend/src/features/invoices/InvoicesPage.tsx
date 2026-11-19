import { useState, type FormEvent } from "react";

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
import type { CounterpartDto } from "@/bindings/CounterpartDto";
import type { InvoiceDto } from "@/bindings/InvoiceDto";
import type { Paged } from "@/bindings/Paged";

interface LineState {
  description: string;
  quantity: string;
  unit_price: string;
  vat_rate: string;
}

function lineAmounts(l: LineState) {
  const qty = Number(l.quantity) || 0;
  const price = Number(l.unit_price) || 0;
  const rate = Number(l.vat_rate) || 0;
  const net = qty * price;
  const vat = (net * rate) / 100;
  return {
    net_amount: net.toFixed(2),
    vat_amount: vat.toFixed(2),
    total_amount: (net + vat).toFixed(2),
  };
}

export default function InvoicesPage() {
  const qc = useQueryClient();
  const list = useQuery({
    queryKey: ["invoices"],
    queryFn: () => api.get<Paged<InvoiceDto>>("/v1/invoices?limit=500"),
  });
  const counterparts = useQuery({
    queryKey: ["counterparts"],
    queryFn: () => api.get<Paged<CounterpartDto>>("/v1/counterparts?limit=500"),
  });

  const [direction, setDirection] = useState("out");
  const [docType, setDocType] = useState("01");
  const [issueDate, setIssueDate] = useState(() =>
    new Date().toISOString().slice(0, 10),
  );
  const [counterpartId, setCounterpartId] = useState("");
  const [lines, setLines] = useState<LineState[]>([
    { description: "", quantity: "1", unit_price: "0", vat_rate: "20" },
  ]);
  const [error, setError] = useState<string | null>(null);

  const refresh = () => void qc.invalidateQueries({ queryKey: ["invoices"] });

  const create = useMutation({
    mutationFn: async () => {
      const cp = counterparts.data?.items.find((c) => c.id === Number(counterpartId));
      const payload = {
        direction,
        document_type: docType,
        issue_date: issueDate,
        counterpart_id: Number(counterpartId),
        counterpart_name: cp?.name ?? "",
        currency: "EUR",
        lines: lines.map((l) => ({ ...l, product_id: 0, ...lineAmounts(l) })),
      };
      return api.post<InvoiceDto>("/v1/invoices", payload);
    },
    onSuccess: refresh,
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const act = useMutation({
    mutationFn: ({ id, path }: { id: number; path: string }) =>
      api.post(`/v1/invoices/${id}/${path}`),
    onSuccess: refresh,
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const storno = useMutation({
    mutationFn: async (inv: InvoiceDto) => {
      const cp = counterparts.data?.items.find((c) => c.id === inv.counterpart_id);
      return api.post<InvoiceDto>("/v1/invoices", {
        direction: inv.direction,
        document_type: "03",
        issue_date: inv.issue_date,
        counterpart_id: inv.counterpart_id,
        counterpart_name: cp?.name ?? inv.counterpart_name,
        original_invoice_id: inv.id,
        currency: inv.currency,
        lines: inv.lines.length
          ? inv.lines
          : [{ description: "черно сторно", quantity: "1", unit_price: "0", vat_rate: "0", product_id: 0, net_amount: "0.00", vat_amount: "0.00", total_amount: "0.00" }],
      });
    },
    onSuccess: (_v, variables) =>
      void qc.invalidateQueries({ queryKey: ["invoices"] }).then(() => act.mutate({ id: variables.id, path: "post" })),
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  function submit(e: FormEvent) {
    e.preventDefault();
    create.mutate();
  }

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Фактури</h1>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">Нова чернова</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={submit} className="flex flex-col gap-4">
            <div className="flex flex-wrap items-end gap-3">
              <div className="flex flex-col gap-1">
                <Label>Посока</Label>
                <select
                  value={direction}
                  onChange={(e) => setDirection(e.target.value)}
                  className="h-9 rounded-md border bg-background px-2 text-sm"
                >
                  <option value="out">изходяща</option>
                  <option value="in">входяща</option>
                </select>
              </div>
              <div className="flex flex-col gap-1">
                <Label>Тип</Label>
                <select
                  value={docType}
                  onChange={(e) => setDocType(e.target.value)}
                  className="h-9 rounded-md border bg-background px-2 text-sm"
                >
                  <option value="01">01 фактура</option>
                  <option value="02">02 дебитно</option>
                  <option value="03">03 кредитно</option>
                  <option value="05">05 входяща</option>
                  <option value="07">07 митница</option>
                </select>
              </div>
              <div className="flex flex-col gap-1">
                <Label>Дата</Label>
                <Input
                  type="date"
                  value={issueDate}
                  onChange={(e) => setIssueDate(e.target.value)}
                  required
                />
              </div>
              <div className="flex min-w-52 flex-col gap-1">
                <Label>Контрагент</Label>
                <select
                  value={counterpartId}
                  onChange={(e) => setCounterpartId(e.target.value)}
                  className="h-9 rounded-md border bg-background px-2 text-sm"
                  required
                >
                  <option value="">—</option>
                  {(counterparts.data?.items ?? []).map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="flex flex-col gap-2">
              {lines.map((l, i) => (
                <div key={i} className="flex flex-wrap items-end gap-2">
                  <div className="flex min-w-52 flex-col gap-1">
                    <Label>Описание</Label>
                    <Input
                      value={l.description}
                      onChange={(e) =>
                        setLines(lines.map((x, j) => (j === i ? { ...x, description: e.target.value } : x)))
                      }
                    />
                  </div>
                  <div className="flex w-20 flex-col gap-1">
                    <Label>Кол.</Label>
                    <Input
                      value={l.quantity}
                      onChange={(e) =>
                        setLines(lines.map((x, j) => (j === i ? { ...x, quantity: e.target.value } : x)))
                      }
                    />
                  </div>
                  <div className="flex w-24 flex-col gap-1">
                    <Label>Цена</Label>
                    <Input
                      value={l.unit_price}
                      onChange={(e) =>
                        setLines(lines.map((x, j) => (j === i ? { ...x, unit_price: e.target.value } : x)))
                      }
                    />
                  </div>
                  <div className="flex w-20 flex-col gap-1">
                    <Label>ДДС %</Label>
                    <Input
                      value={l.vat_rate}
                      onChange={(e) =>
                        setLines(lines.map((x, j) => (j === i ? { ...x, vat_rate: e.target.value } : x)))
                      }
                    />
                  </div>
                  <span className="pb-2 text-sm">{lineAmounts(l).total_amount}</span>
                  <Button
                    type="button"
                    variant="ghost"
                    onClick={() => setLines(lines.filter((_, j) => j !== i))}
                  >
                    ✕
                  </Button>
                </div>
              ))}
              <Button
                type="button"
                variant="outline"
                className="w-fit"
                onClick={() =>
                  setLines([...lines, { description: "", quantity: "1", unit_price: "0", vat_rate: "20" }])
                }
              >
                + ред
              </Button>
            </div>

            <Button type="submit" className="w-fit" disabled={create.isPending}>
              Създай чернова
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-4">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Номер</TableHead>
                <TableHead>Дата</TableHead>
                <TableHead>Контрагент</TableHead>
                <TableHead>Статус</TableHead>
                <TableHead className="text-right">Сума</TableHead>
                <TableHead>Действия</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(list.data?.items ?? []).map((inv) => (
                <TableRow key={inv.id}>
                  <TableCell>{inv.number}</TableCell>
                  <TableCell>{inv.issue_date}</TableCell>
                  <TableCell>{inv.counterpart_name}</TableCell>
                  <TableCell>{inv.status}</TableCell>
                  <TableCell className="text-right">{inv.total_amount ?? ""}</TableCell>
                  <TableCell className="flex gap-2">
                    {inv.status === "draft" ? (
                      <Button size="sm" onClick={() => act.mutate({ id: inv.id, path: "post" })}>
                        Пост
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => act.mutate({ id: inv.id, path: "unpost" })}
                      >
                        Отпост
                      </Button>
                    )}
                    {inv.status === "posted" && inv.document_type === "01" ? (
                      <Button
                        size="sm"
                        variant="destructive"
                        onClick={() => {
                          void storno.mutate(inv);
                        }}
                      >
                        Сторно
                      </Button>
                    ) : null}
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
