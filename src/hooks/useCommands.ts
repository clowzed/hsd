import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "@/store/useAppStore";
import type { PdfRecord, PrinterInfo, AppMode, AppSettings } from "@/types";
import { useCallback } from "react";

export function useCommands() {
  const generatePdf = useCallback(async () => {
    useAppStore.getState().setGeneratingPdf(true);
    try {
      const record = await invoke<PdfRecord>("generate_pdf");
      useAppStore.getState().addPdfRecord(record);
    } catch (e) {
      useAppStore.getState().setError(String(e));
    } finally {
      useAppStore.getState().setGeneratingPdf(false);
    }
  }, []);

  const clearBuffer = useCallback(async () => {
    try {
      await invoke("clear_buffer");
      useAppStore.getState().clearBuffer();
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  const removeCode = useCallback(async (index: number) => {
    try {
      await invoke("remove_code", { index });
      useAppStore.getState().removeCode(index);
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  const listPdfs = useCallback(async () => {
    try {
      const history = await invoke<PdfRecord[]>("list_pdfs");
      useAppStore.getState().setPdfHistory(history);
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  const clearPdfHistory = useCallback(async () => {
    try {
      await invoke("clear_pdf_history");
      useAppStore.getState().clearPdfHistory();
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  const openPdf = useCallback(async (path: string) => {
    try {
      await invoke("open_pdf", { path });
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  const listPrinters = useCallback(async () => {
    try {
      const printers = await invoke<PrinterInfo[]>("list_printers");
      useAppStore.getState().setPrinters(printers);
      // Auto-select default printer if none selected
      const current = useAppStore.getState().selectedPrinter;
      if (!current) {
        const defaultPrinter = printers.find((p) => p.is_default);
        if (defaultPrinter) {
          useAppStore.getState().setPrinter(defaultPrinter.name);
        }
      }
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  const printPdf = useCallback(async (path: string) => {
    const printer = useAppStore.getState().selectedPrinter;
    if (!printer) {
      useAppStore.getState().setError("Выберите принтер");
      return;
    }
    useAppStore.getState().setPrinting(true);
    try {
      await invoke("print_pdf", { path, printerName: printer });
    } catch (e) {
      useAppStore.getState().setError(String(e));
    } finally {
      useAppStore.getState().setPrinting(false);
    }
  }, []);

  const setMode = useCallback(async (mode: AppMode) => {
    try {
      await invoke("set_mode", { mode });
      useAppStore.getState().setMode(mode);
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  const setPrinter = useCallback(async (printerName: string) => {
    try {
      await invoke("set_printer", { printerName });
      useAppStore.getState().setPrinter(printerName);
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  const getSettings = useCallback(async () => {
    try {
      const settings = await invoke<AppSettings>("get_settings");
      useAppStore.getState().setMode(settings.mode);
      if (settings.selected_printer) {
        useAppStore.getState().setPrinter(settings.selected_printer);
      }
      useAppStore.getState().setBarcodeEnabled(settings.barcode_enabled);
      useAppStore.getState().setBarcodeCopies(settings.barcode_copies);
      useAppStore.getState().setBarcodeActivePreset(settings.barcode_active_preset);
      useAppStore.getState().setBarcodePresets(settings.barcode_presets);
      useAppStore.getState().setDuplicateDetectionBuffered(settings.duplicate_detection_buffered);
      useAppStore.getState().setDuplicateDetectionInstant(settings.duplicate_detection_instant);
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  const setBarcodeSettings = useCallback(
    async (
      enabled: boolean,
      copies: number,
      activePreset: string | null
    ) => {
      try {
        await invoke("set_barcode_settings", {
          enabled,
          copies,
          activePreset,
        });
        useAppStore.getState().setBarcodeEnabled(enabled);
        useAppStore.getState().setBarcodeCopies(copies);
        useAppStore.getState().setBarcodeActivePreset(activePreset);
      } catch (e) {
        useAppStore.getState().setError(String(e));
      }
    },
    []
  );

  const setBarcodePresetDirectory = useCallback(
    async (presetName: string, directory: string) => {
      try {
        await invoke("set_barcode_preset_directory", {
          presetName,
          directory,
        });
        useAppStore.getState().updatePresetDirectory(presetName, directory);
      } catch (e) {
        useAppStore.getState().setError(String(e));
      }
    },
    []
  );

  const selectDirectory = useCallback(async (): Promise<string | null> => {
    try {
      return await invoke<string | null>("select_directory");
    } catch (e) {
      useAppStore.getState().setError(String(e));
      return null;
    }
  }, []);

  const printBufferedBarcodes = useCallback(async () => {
    try {
      await invoke<number>("print_buffered_barcodes");
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  const setDuplicateDetection = useCallback(
    async (buffered: boolean, instant: boolean) => {
      try {
        await invoke("set_duplicate_detection", { buffered, instant });
        useAppStore.getState().setDuplicateDetectionBuffered(buffered);
        useAppStore.getState().setDuplicateDetectionInstant(instant);
      } catch (e) {
        useAppStore.getState().setError(String(e));
      }
    },
    []
  );

  const clearScanHistory = useCallback(async () => {
    try {
      await invoke("clear_scan_history");
    } catch (e) {
      useAppStore.getState().setError(String(e));
    }
  }, []);

  return {
    generatePdf,
    clearBuffer,
    removeCode,
    listPdfs,
    clearPdfHistory,
    openPdf,
    listPrinters,
    printPdf,
    setMode,
    setPrinter,
    getSettings,
    setBarcodeSettings,
    setBarcodePresetDirectory,
    selectDirectory,
    printBufferedBarcodes,
    setDuplicateDetection,
    clearScanHistory,
  };
}
