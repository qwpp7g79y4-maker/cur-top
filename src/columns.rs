//! CUR 2.0 column names from the AWS Data Exports table dictionary, read 2026-11-09.
//!
//! - <https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2.html>
//! - <https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2-line-item.html>
//! - <https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2-bill.html>
//! - <https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2-resource-tags.html>
//! - Delivery names `.csv.gz` and `.snappy.parquet`:
//!   <https://docs.aws.amazon.com/cur/latest/userguide/dataexports-export-delivery.html>

use crate::error::Error;

pub const DICTIONARY_URL: &str =
    "https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2.html";
pub const DICTIONARY_DATE: &str = "2026-11-09";
pub const LINE_ITEM_DICTIONARY_URL: &str =
    "https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2-line-item.html";
pub const BILL_DICTIONARY_URL: &str =
    "https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2-bill.html";
pub const RESOURCE_TAGS_DICTIONARY_URL: &str =
    "https://docs.aws.amazon.com/cur/latest/userguide/table-dictionary-cur2-resource-tags.html";

pub(crate) const BILL_BILLING_PERIOD_START_DATE: &str = "bill_billing_period_start_date";
pub(crate) const LINE_ITEM_PRODUCT_CODE: &str = "line_item_product_code";
pub(crate) const LINE_ITEM_USAGE_TYPE: &str = "line_item_usage_type";
pub(crate) const LINE_ITEM_UNBLENDED_COST: &str = "line_item_unblended_cost";
pub(crate) const LINE_ITEM_BLENDED_COST: &str = "line_item_blended_cost";
pub(crate) const LINE_ITEM_NET_UNBLENDED_COST: &str = "line_item_net_unblended_cost";
pub(crate) const RESOURCE_TAGS: &str = "resource_tags";

/// The three line-item cost columns. Rates and usage amounts are not costs.
pub fn parse_amount_column(raw: &str) -> Result<&'static str, Error> {
    match raw {
        LINE_ITEM_UNBLENDED_COST => Ok(LINE_ITEM_UNBLENDED_COST),
        LINE_ITEM_BLENDED_COST => Ok(LINE_ITEM_BLENDED_COST),
        LINE_ITEM_NET_UNBLENDED_COST => Ok(LINE_ITEM_NET_UNBLENDED_COST),
        _ => Err(Error::new("invalid amount column")),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_amount_column, BILL_BILLING_PERIOD_START_DATE, BILL_DICTIONARY_URL, DICTIONARY_DATE,
        DICTIONARY_URL, LINE_ITEM_BLENDED_COST, LINE_ITEM_DICTIONARY_URL,
        LINE_ITEM_NET_UNBLENDED_COST, LINE_ITEM_PRODUCT_CODE, LINE_ITEM_UNBLENDED_COST,
        LINE_ITEM_USAGE_TYPE, RESOURCE_TAGS, RESOURCE_TAGS_DICTIONARY_URL,
    };

    #[test]
    fn claim_c2_official_column_names_and_dictionary_date() {
        assert_eq!(
            BILL_BILLING_PERIOD_START_DATE,
            "bill_billing_period_start_date"
        );
        assert_eq!(LINE_ITEM_PRODUCT_CODE, "line_item_product_code");
        assert_eq!(LINE_ITEM_USAGE_TYPE, "line_item_usage_type");
        assert_eq!(LINE_ITEM_UNBLENDED_COST, "line_item_unblended_cost");
        assert_eq!(LINE_ITEM_BLENDED_COST, "line_item_blended_cost");
        assert_eq!(LINE_ITEM_NET_UNBLENDED_COST, "line_item_net_unblended_cost");
        assert_eq!(RESOURCE_TAGS, "resource_tags");
        assert_eq!(DICTIONARY_DATE, "2026-11-09");
        assert!(DICTIONARY_URL.contains("table-dictionary-cur2.html"));
        assert!(LINE_ITEM_DICTIONARY_URL.contains("table-dictionary-cur2-line-item"));
        assert!(BILL_DICTIONARY_URL.contains("table-dictionary-cur2-bill"));
        assert!(RESOURCE_TAGS_DICTIONARY_URL.contains("table-dictionary-cur2-resource-tags"));
    }

    #[test]
    fn claim_c3_amount_columns_are_the_three_cost_fields() {
        assert_eq!(
            parse_amount_column("line_item_unblended_cost").unwrap(),
            "line_item_unblended_cost"
        );
        assert_eq!(
            parse_amount_column("line_item_blended_cost").unwrap(),
            "line_item_blended_cost"
        );
        assert_eq!(
            parse_amount_column("line_item_net_unblended_cost").unwrap(),
            "line_item_net_unblended_cost"
        );
        let err = parse_amount_column("line_item_unblended_rate").unwrap_err();
        assert_eq!(err.message(), "invalid amount column");
        assert!(!err.message().contains("unblended_rate"));
        let err = parse_amount_column("line_item_usage_amount").unwrap_err();
        assert_eq!(err.message(), "invalid amount column");
        assert!(!err.message().contains("usage_amount"));
    }
}
