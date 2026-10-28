//! Count markers and takeoff rows (BuildCraft): place a marker, list the totals per item, list
//! every measurement and count with its breakdown, and change breakdowns.
use crate::{Args, Automation, Result, failed};
use pdfcraft_engine::{
    Edit,
    measure::{
        count::NewCount,
        takeoff::{self, Breakdown, Field, GroupBy, Markup, NewTakeoff, Patch},
    },
};
use serde_json::{Value, json};

/// The breakdown fields given in `a`, each `Some` when present (an empty string clears it).
fn patch(a: &Args) -> Result<Patch> {
    let get = |f: Field| a.opt_str(f.key()).map(|v| v.map(str::to_owned));
    Ok(Patch {
        building: get(Field::Building)?,
        floor: get(Field::Floor)?,
        area: get(Field::Area)?,
        system: get(Field::System)?,
        bid_item: get(Field::BidItem)?,
    })
}

fn breakdown_json(b: &Breakdown) -> Value {
    Value::Object(Field::ALL.iter().map(|f| (f.key().to_owned(), json!(b.get(*f)))).collect())
}

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
        let marker = NewCount {
            page,
            at,
            item: a.str("item")?.into(),
            color,
            size: a.opt_num("size")?.unwrap_or(10.0),
            author: a.opt_str("author")?.unwrap_or(super::comments::DEFAULT_AUTHOR).into(),
        };
        let breakdown = patch(a)?.apply(&Breakdown::default());
        self.apply(a, Edit::AddTakeoff(NewTakeoff { markup: Markup::Count(marker), breakdown }))
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
    /// Every measurement and count with its breakdown; with `group_by`, grouped with totals.
    pub(crate) fn takeoff_list(&self, a: &Args) -> Result<Value> {
        let doc = self.doc(a)?;
        let listed = doc.takeoff_rows().map_err(failed)?;
        let by = match a.opt_str("group_by")? {
            None => None,
            Some("item") => Some(GroupBy::Item),
            Some(k) => Some(GroupBy::Field(Field::from_key(k).ok_or_else(|| failed("group_by is item, building, floor, area, system or bid_item"))?)),
        };
        let rows: Vec<Value> = listed
            .rows
            .iter()
            .map(|r| {
                json!({"page":r.page.saturating_add(1),"index":r.index.saturating_add(1),"id":r.id,"type":r.kind,"item":r.item,
                    "value":r.value,"unit":r.unit,"quantity":r.quantity,"breakdown":breakdown_json(&r.breakdown),"author":r.author})
            })
            .collect();
        let mut out = json!({"rows":rows,"unsupported":listed.unsupported,"truncated":listed.truncated});
        if let Some(by) = by {
            let groups: Vec<Value> = takeoff::group(&listed.rows, by)
                .into_iter()
                .map(|g| {
                    let totals: Vec<Value> = g.totals.iter().map(|(u, v)| json!({"unit":u,"value":v,"text":takeoff::total_text(u, *v)})).collect();
                    let rows: Vec<usize> = g.rows.iter().map(|i| i.saturating_add(1)).collect();
                    json!({"key":g.key,"rows":rows,"totals":totals})
                })
                .collect();
            out["groups"] = json!(groups);
        }
        Ok(out)
    }
    /// Set breakdown fields on measurements and counts given as `markups` [[page, index], …].
    pub(crate) fn takeoff_breakdown(&mut self, a: &Args) -> Result<Value> {
        let v = a.get("markups").ok_or_else(|| Args::missing("markups"))?;
        let list = v
            .as_array()
            .filter(|l| !l.is_empty() && l.len() <= takeoff::MAX_TARGETS)
            .ok_or_else(|| Args::wrong("markups", "1 to 10,000 [page, index] pairs"))?;
        let targets = list
            .iter()
            .map(|p| {
                let pair = p.as_array().filter(|p| p.len() == 2).ok_or_else(|| Args::wrong("markups", "[page, index] pairs"))?;
                let n = |i: usize| pair.get(i).and_then(Value::as_u64).filter(|n| *n >= 1).and_then(|n| usize::try_from(n - 1).ok());
                n(0).zip(n(1)).ok_or_else(|| Args::wrong("markups", "1-based [page, index] pairs"))
            })
            .collect::<Result<Vec<_>>>()?;
        let patch = patch(a)?;
        if patch == Patch::default() {
            return Err(failed("give at least one of building, floor, area, system or bid_item"));
        }
        self.apply(a, Edit::SetBreakdown { targets, patch })
    }
}
