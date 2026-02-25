import { create } from "zustand";
import type {
  ScannerStatus,
  ScannedCode,
  LastScanResult,
  PdfRecord,
  CrptResponse,
  AppMode,
  PrinterInfo,
} from "@/types";

interface AppState {
  scannerStatus: ScannerStatus;
  scannedCodes: ScannedCode[];
  lastScanResult: LastScanResult;
  pdfHistory: PdfRecord[];
  currentError: string | null;
  isGeneratingPdf: boolean;

  appMode: AppMode;
  selectedPrinter: string | null;
  printers: PrinterInfo[];
  isPrinting: boolean;
  currentBufferedPdf: PdfRecord | null;

  setScannerStatus: (s: ScannerStatus) => void;
  setScanStarted: () => void;
  setScanSuccess: (code: ScannedCode, response: CrptResponse) => void;
  setScanError: (message: string, explanation: string) => void;
  addPdfRecord: (record: PdfRecord) => void;
  setPdfHistory: (history: PdfRecord[]) => void;
  setError: (error: string | null) => void;
  clearBuffer: () => void;
  clearPdfHistory: () => void;
  setGeneratingPdf: (v: boolean) => void;

  setMode: (mode: AppMode) => void;
  setPrinter: (printer: string | null) => void;
  setPrinters: (list: PrinterInfo[]) => void;
  setPrinting: (v: boolean) => void;
  setBufferedPdf: (record: PdfRecord | null) => void;
  removeCode: (index: number) => void;
}

export const useAppStore = create<AppState>((set) => ({
  scannerStatus: { type: "Connecting" },
  scannedCodes: [],
  lastScanResult: { type: "None" },
  pdfHistory: [],
  currentError: null,
  isGeneratingPdf: false,

  appMode: "Buffered",
  selectedPrinter: null,
  printers: [],
  isPrinting: false,
  currentBufferedPdf: null,

  setScannerStatus: (s) => set({ scannerStatus: s }),

  setScanStarted: () =>
    set({ lastScanResult: { type: "Validating" }, currentError: null }),

  setScanSuccess: (code, response) =>
    set((state) => ({
      scannedCodes: [...state.scannedCodes, code],
      lastScanResult: { type: "Success", code, response },
      currentError: null,
    })),

  setScanError: (message, explanation) =>
    set({
      lastScanResult: { type: "Error", message, explanation },
      currentError: `${message}: ${explanation}`,
    }),

  addPdfRecord: (record) =>
    set((state) => ({
      pdfHistory: [record, ...state.pdfHistory],
    })),

  setPdfHistory: (history) => set({ pdfHistory: history }),
  setError: (error) => set({ currentError: error }),

  clearBuffer: () =>
    set({
      scannedCodes: [],
      lastScanResult: { type: "None" },
      currentError: null,
      currentBufferedPdf: null,
    }),

  clearPdfHistory: () => set({ pdfHistory: [] }),
  setGeneratingPdf: (v) => set({ isGeneratingPdf: v }),

  setMode: (mode) => set({ appMode: mode }),
  setPrinter: (printer) => set({ selectedPrinter: printer }),
  setPrinters: (list) => set({ printers: list }),
  setPrinting: (v) => set({ isPrinting: v }),
  setBufferedPdf: (record) => set({ currentBufferedPdf: record }),
  removeCode: (index) =>
    set((state) => ({
      scannedCodes: state.scannedCodes.filter((_, i) => i !== index),
    })),
}));
