import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "@/store/useAppStore";
import type {
  ScannerStatus,
  ScanSuccessPayload,
  ScanErrorPayload,
  PdfRecord,
  InstantPrintPayload,
} from "@/types";

export function useTauriEvents() {
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    const setup = async () => {
      unlisteners.push(
        await listen<ScannerStatus>("scanner-status-changed", (e) => {
          useAppStore.getState().setScannerStatus(e.payload);
        })
      );

      unlisteners.push(
        await listen<void>("scan-started", () => {
          useAppStore.getState().setScanStarted();
        })
      );

      unlisteners.push(
        await listen<ScanSuccessPayload>("scan-success", (e) => {
          useAppStore
            .getState()
            .setScanSuccess(e.payload.code, e.payload.response);
        })
      );

      unlisteners.push(
        await listen<ScanErrorPayload>("scan-error", (e) => {
          useAppStore
            .getState()
            .setScanError(e.payload.message, e.payload.explanation);
        })
      );

      unlisteners.push(
        await listen<string>("error", (e) => {
          useAppStore.getState().setError(e.payload);
        })
      );

      unlisteners.push(
        await listen<PdfRecord | null>("pdf-regenerated", (e) => {
          useAppStore.getState().setBufferedPdf(e.payload);
        })
      );

      unlisteners.push(
        await listen<InstantPrintPayload>("instant-print-success", (e) => {
          useAppStore
            .getState()
            .setError(null);
          // Optionally show a brief notification via lastScanResult
          // The scan-success event already handles the UI update
        })
      );
    };

    setup();

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, []);
}
