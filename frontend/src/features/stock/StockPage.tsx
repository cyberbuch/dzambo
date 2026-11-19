import { useQuery } from "@tanstack/react-query";

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
import type { StockItemDto } from "@/bindings/StockItemDto";

export default function StockPage() {
  const report = useQuery({
    queryKey: ["stock-report"],
    queryFn: () => api.get<StockItemDto[]>("/v1/stock/report"),
  });

  const total = (report.data ?? []).reduce((s, r) => s + Number(r.value), 0);

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Склад</h1>

      <Card>
        <CardHeader className="flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            Наличност по партиди (WAC)
          </CardTitle>
          <span className="text-sm font-medium">Обща стойност: {total.toFixed(2)}</span>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Стока</TableHead>
                <TableHead>Код</TableHead>
                <TableHead>Ед.</TableHead>
                <TableHead className="text-right">Количество</TableHead>
                <TableHead className="text-right">Ср. цена</TableHead>
                <TableHead className="text-right">Стойност</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(report.data ?? []).map((r) => (
                <TableRow key={r.product_id}>
                  <TableCell>{r.name}</TableCell>
                  <TableCell>{r.code}</TableCell>
                  <TableCell>{r.unit}</TableCell>
                  <TableCell className="text-right">{r.quantity}</TableCell>
                  <TableCell className="text-right">{r.avg_unit_cost}</TableCell>
                  <TableCell className="text-right">{r.value}</TableCell>
                </TableRow>
              ))}
              {!(report.data?.length) ? (
                <TableRow>
                  <TableCell colSpan={6} className="text-muted-foreground">
                    Няма наличности — закупени стоки първо стават партиди.
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
