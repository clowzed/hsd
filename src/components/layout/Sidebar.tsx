import { useAppStore } from "@/store/useAppStore";
import { useCommands } from "@/hooks/useCommands";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { ExternalLink, Trash2, FileText } from "lucide-react";

export function Sidebar() {
  const pdfHistory = useAppStore((s) => s.pdfHistory);
  const { clearPdfHistory, openPdf } = useCommands();

  return (
    <aside className="w-52 shrink-0 border-l bg-card flex flex-col">
      <div className="px-4 py-3 border-b">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          История PDF
        </h2>
      </div>

      {pdfHistory.length === 0 ? (
        <div className="flex-1 flex flex-col items-center justify-center gap-2 px-4 text-muted-foreground">
          <FileText className="w-8 h-8 opacity-20" />
          <span className="text-xs text-center">Нет созданных PDF</span>
        </div>
      ) : (
        <>
          <ScrollArea className="flex-1">
            <div className="divide-y divide-border">
              {pdfHistory.map((record, i) => (
                <div
                  key={i}
                  className="flex items-center gap-2 px-4 py-3 hover:bg-muted/40 transition-colors group"
                >
                  <div className="flex-1 min-w-0">
                    <p className="text-xs font-medium truncate text-foreground">
                      {record.filename}
                    </p>
                    <p className="text-[11px] text-muted-foreground mt-0.5">
                      {record.created_at}
                      {record.code_count > 0 && (
                        <span className="ml-1.5">
                          &middot; {record.code_count} шт.
                        </span>
                      )}
                    </p>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 opacity-0 group-hover:opacity-100 transition-opacity shrink-0 text-muted-foreground hover:text-foreground"
                    onClick={() => openPdf(record.path)}
                  >
                    <ExternalLink className="w-3.5 h-3.5" />
                  </Button>
                </div>
              ))}
            </div>
          </ScrollArea>

          <div className="p-3 border-t">
            <Button
              variant="outline"
              size="sm"
              className="w-full gap-1.5 text-red-500 dark:text-red-400 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-950/40 border-red-200 dark:border-red-800"
              onClick={clearPdfHistory}
            >
              <Trash2 className="w-3.5 h-3.5" />
              Очистить историю
            </Button>
          </div>
        </>
      )}
    </aside>
  );
}
