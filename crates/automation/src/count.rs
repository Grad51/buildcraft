//! Count markers (BuildCraft): place one, and list the totals per item.
use crate::{Args, Automation, Result, failed};
use pdfcraft_engine::{Edit, measure::count::NewCount};
use serde_json::{Value, json};

impl Automation {
    pub(crate) fn count_add(&mut self, a: &Args) -> Result<Value> {
        let page = self.page(a)?;
        let at = a.nums::<2>("at")?.ok_or_else(|| failed("a count marker needs at"))?;
        if at.iter().any(|v| !v.is_finite() || v.abs() > 1e9) {
            return Err(failed("count coordinates must be finite and within 1e9 points"));
        }
        let at = self.doc(a)?.measurement_to_user(page, at).map_err(failed)?;
        let color = match a.get("color") {
            None => [0.86, 0.15, 0.15],
            Some(_) => a.nums::<3>("color")?.ok_or_else(|| failed("color is [r, g, b] from 0 to 1"))?,
        };
        let edit = Edit::AddCount(NewCount {
            page,
            at,
            item: a.str("item")?.into(),
            color,
            size: a.opt_num("size")?.unwrap_or(10.0),
            author: a.opt_str("author")?.unwrap_or(super::comments::DEFAULT_AUTHOR).into(),
        });
        self.apply(a, edit)
    }
    pub(crate) fn count_list(&self, a: &Args) -> Result<Value> {
        let doc = self.doc(a)?;
        let listing = doc.counts().map_err(failed)?;
        let only = a.opt_str("item")?.map(str::trim);
        let markers: Vec<Value> = listing
            .markers
            .iter()
            .filter(|m| only.is_none_or(|i| m.item == i))
            .map(|m| -> Result<Value> {
                let at = doc.measurement_to_view(m.page, m.at).map_err(failed)?;
                Ok(json!({"page":m.page.saturating_add(1),"index":m.index.saturating_add(1),"id":m.id,"item":m.item,"at":at}))
            })
            .collect::<Result<_>>()?;
        let totals: Vec<Value> = listing
            .totals
            .iter()
            .filter(|t| only.is_none_or(|i| t.item == i))
            .map(|t| {
                let pages: serde_json::Map<String, Value> = t.pages.iter().map(|(p, n)| (p.saturating_add(1).to_string(), json!(n))).collect();
                json!({"item":t.item,"count":t.count,"pages":pages})
            })
            .collect();
        Ok(json!({"totals":totals,"markers":markers,"truncated":listing.truncated}))
    }
}
