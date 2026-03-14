import { create } from "zustand";
import type {
  ScannerStatus,
  ScannedCode,
  LastScanResult,
  PdfRecord,
  CrptResponse,
  AppMode,
  PrinterInfo,
  BarcodePreset,
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

  barcodeEnabled: boolean;
  barcodeCopies: number;
  barcodeActivePreset: string | null;
  barcodePresets: BarcodePreset[];

  duplicateDetectionBuffered: boolean;
  duplicateDetectionInstant: boolean;

  devMode: boolean;
  updaterLogs: string[];

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

  setBarcodeEnabled: (v: boolean) => void;
  setBarcodeCopies: (n: number) => void;
  setBarcodeActivePreset: (name: string | null) => void;
  setBarcodePresets: (presets: BarcodePreset[]) => void;
  updatePresetDirectory: (name: string, directory: string) => void;

  setDuplicateDetectionBuffered: (v: boolean) => void;
  setDuplicateDetectionInstant: (v: boolean) => void;

  setDevMode: (v: boolean) => void;
  addUpdaterLog: (msg: string) => void;
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

  barcodeEnabled: false,
  barcodeCopies: 1,
  barcodeActivePreset: null,
  barcodePresets: [],

  duplicateDetectionBuffered: true,
  duplicateDetectionInstant: false,

  devMode: false,
  updaterLogs: [],

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

  setBarcodeEnabled: (v) => set({ barcodeEnabled: v }),
  setBarcodeCopies: (n) => set({ barcodeCopies: n }),
  setBarcodeActivePreset: (name) => set({ barcodeActivePreset: name }),
  setBarcodePresets: (presets) => set({ barcodePresets: presets }),
  updatePresetDirectory: (name, directory) =>
    set((state) => ({
      barcodePresets: state.barcodePresets.map((p) =>
        p.name === name ? { ...p, directory } : p
      ),
    })),

  setDuplicateDetectionBuffered: (v) => set({ duplicateDetectionBuffered: v }),
  setDuplicateDetectionInstant: (v) => set({ duplicateDetectionInstant: v }),

  setDevMode: (v) => set({ devMode: v }),
  addUpdaterLog: (msg) =>
    set((state) => {
      const logs = [
        ...state.updaterLogs,
        `${new Date().toLocaleTimeString()} ${msg}`,
      ];
      return { updaterLogs: logs.slice(-50) };
    }),
}));
