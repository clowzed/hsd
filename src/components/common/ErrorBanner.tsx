import { X, AlertCircle } from "lucide-react";
import { Button } from "@/components/ui/button";

interface Props {
  message: string;
  onDismiss: () => void;
}

export function ErrorBanner({ message, onDismiss }: Props) {
  return (
    <div className="flex items-start gap-3 px-4 py-3 bg-red-50 dark:bg-red-950/40 border border-red-200 dark:border-red-800 rounded-lg text-sm">
      <AlertCircle className="w-4 h-4 text-red-500 dark:text-red-400 mt-0.5 shrink-0" />
      <span className="flex-1 text-red-700 dark:text-red-300">{message}</span>
      <Button
        variant="ghost"
        size="icon"
        className="h-5 w-5 text-red-400 hover:text-red-600 hover:bg-red-100 dark:hover:bg-red-900/40 shrink-0 -mt-0.5"
        onClick={onDismiss}
      >
        <X className="w-3.5 h-3.5" />
      </Button>
    </div>
  );
}
