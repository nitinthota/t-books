//! Payments stay on the same voucher. At most five PI / payment blocks.
//! Matches loopbook `voucher-data.ts` totals: paid + TDS = payment.

use super::status::{is_na_tax_inv, payment_status, tax_flag_for, TaxFlag};

pub const MAX_PAYMENT_BLOCKS: usize = 5;
