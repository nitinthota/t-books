export const APP_NAME = "T Books";
export const APP_VERSION = "1.0.0";
export const APP_MONOGRAM = "T";

/** Windows: %LOCALAPPDATA%\T-Books\tbooks.db */
export const DATA_FOLDER_NAME = "T-Books";
export const DB_FILE_NAME = "tbooks.db";
export const CREDENTIALS_FILE_NAME = "credentials.json";
export const IDB_NAME = "t-books";
export const IDB_STORE = "kv";
export const IDB_SQLITE_KEY = "tbooks.db";

export const OWNER_EMAIL = "thotanitin123@gmail.com";
export const OWNER_ROLE = "owner" as const;

export const ACCESS_TAB = "Access";
export const VOUCHER_RAW_TAB = "Voucher_Raw_Data";
export const DEFAULT_SPREADSHEET_ID = "1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE";

export const OFFLINE_BANNER =
  "Offline — working on this PC. Access list and Refresh paused.";
export const CREDENTIALS_BANNER = "Google credentials not found. Running offline.";
export const CREDENTIALS_MISSING_ERROR =
  "Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.";
export const OFFLINE_REFRESH_ERROR = "This PC is offline. Cannot refresh the Access list.";
export const OFFLINE_VOUCHER_REFRESH_ERROR = "This PC is offline. Cannot refresh vouchers.";
export const REFRESH_SUCCESS_TOAST = "Data refreshed";

export const MIN_PASSWORD_LENGTH = 8;
export const SCHEMA_VERSION = 6;
export const OFFLINE_SUBMIT_ERROR = "This PC is offline. Cannot submit to Google.";
export const SUBMIT_SUCCESS_TOAST = "Submitted to Google.";

export { LOOPBOOK_LOGIC_VERSION } from "./business_rules/index.ts";

/** Match argon2 0.5 default (Argon2id). */
export const ARGON2_MEMORY_KIB = 19_456;
export const ARGON2_ITERATIONS = 2;
export const ARGON2_PARALLELISM = 1;
export const ARGON2_HASH_LENGTH = 32;
