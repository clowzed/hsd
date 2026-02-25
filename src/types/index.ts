// Mirrors of Rust IPC types

export type ScannerStatus =
  | { type: "Disconnected" }
  | { type: "Connecting" }
  | { type: "Connected" }
  | { type: "Error"; message: string };

export interface HonestSignCode {
  raw: number[];
  raw_string: string;
  gtin: string;
  serial: string | null;
  crypto: string | null;
}

export interface ScannedCode {
  code: HonestSignCode;
  product_name: string;
  gtin: string;
  produced_date: string | null;
  expire_date: string | null;
  vendor_code: string | null;
}

export interface CrptResponse {
  id: number | null;
  code_founded: boolean;
  status: string | null;
  status_v2: string | null;
  verified: boolean | null;
  known: boolean | null;
  category: string | null;
  code: string | null;
  gtin: string | null;
  serial: string | null;
  product_name: string | null;
  outer_status: string | null;
  emission_type: string | null;
  pack_type: string | null;
  withdraw_reason: string | null;
  produced_date: number | null;
  introduced_date: number | null;
  expire_date: number | null;
  is_blocked: boolean | null;
}

export type LastScanResult =
  | { type: "None" }
  | { type: "Validating" }
  | { type: "Success"; code: ScannedCode; response: CrptResponse }
  | { type: "Error"; message: string; explanation: string };

export interface PdfRecord {
  path: string;
  filename: string;
  created_at: string;
  code_count: number;
}

export type AppMode = "Buffered" | "Instant";

export interface PrinterInfo {
  name: string;
  is_default: boolean;
}

export interface AppSettings {
  mode: AppMode;
  selected_printer: string | null;
}

export interface InstantPrintPayload {
  filename: string;
  printer: string;
}

export interface ScanSuccessPayload {
  code: ScannedCode;
  response: CrptResponse;
}

export interface ScanErrorPayload {
  message: string;
  explanation: string;
}
