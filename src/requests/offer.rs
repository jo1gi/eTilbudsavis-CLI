use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDate, Utc};
use comfy_table::{Cell, CellAlignment};
use serde::{Deserialize, Serialize};

use super::userdata::UserData;

#[derive(Debug, Deserialize, Serialize, PartialOrd)]
pub(crate) struct Offer {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) dealer: String,
    pub(crate) price: f64,
    pub(crate) cost_per_unit: f64,
    pub(crate) unit: String,
    pub(crate) min_size: f64,
    pub(crate) max_size: f64,
    pub(crate) min_amount: u32,
    pub(crate) max_amount: u32,
    pub(crate) run_from: NaiveDate,
    pub(crate) run_till: NaiveDate,
}

impl PartialEq for Offer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            || (self.dealer == other.dealer
                && self.name == other.name
                && self.run_from == other.run_from
                && self.run_till == other.run_till)
    }
}

impl Eq for Offer {}

impl Default for Offer {
    fn default() -> Self {
        Offer {
            id: String::default(),
            name: String::default(),
            dealer: String::default(),
            price: f64::default(),
            cost_per_unit: f64::default(),
            unit: String::default(),
            min_size: f64::default(),
            max_size: f64::default(),
            min_amount: u32::default(),
            max_amount: u32::default(),
            run_from: Utc::now().date_naive(),
            run_till: Utc::now().date_naive(),
        }
    }
}

impl std::fmt::Display for Offer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let offer_str = format!(
            "{} - {}: {} - {}: {} kr. - {:.2} kr/{}",
            self.run_from.format("%d/%m"),
            self.run_till.format("%d/%m"),
            self.dealer,
            self.name,
            self.price,
            self.cost_per_unit,
            self.unit
        );
        write!(f, "{}", offer_str)?;
        Ok(())
    }
}

impl Offer {
    pub(crate) fn to_table_entry(&self) -> Vec<Cell> {
        let period = format!(
            "{} - {}",
            self.run_from.format("%d/%m"),
            self.run_till.format("%d/%m")
        );
        let cost_per_unit = format!("{:.2} kr/{}", self.cost_per_unit, self.unit);
        let price = format!("{:.2} kr", self.price);

        vec![
            Cell::new(period).set_alignment(CellAlignment::Center),
            Cell::new(self.dealer.to_string()),
            Cell::new(self.name.to_string()),
            Cell::new(price).set_alignment(CellAlignment::Right),
            Cell::new(cost_per_unit).set_alignment(CellAlignment::Right),
        ]
    }
}

pub(crate) async fn retrieve_offers(
    userdata: &mut UserData,
    favorites_changed: bool,
) -> Vec<Offer> {
    match retrieve_cached_offers() {
        Ok(cached_offers) => {
            let cache_outdated = userdata.should_update_cache();
            if favorites_changed || cache_outdated {
                let offers = retrieve_offers_from_remote(userdata).await;
                if let Err(err) = cache_retrieved_offers(userdata, &offers) {
                    eprintln!("{err}");
                }
                offers
            } else {
                cached_offers
            }
        }
        Err(_) => {
            let offers = retrieve_offers_from_remote(userdata).await;
            if let Err(err) = cache_retrieved_offers(userdata, &offers) {
                eprintln!("{err}");
            }
            offers
        }
    }
}

fn cache_retrieved_offers(userdata: &mut UserData, offers: &Vec<Offer>) -> Result<()> {
    let path = dirs::cache_dir()
        .ok_or(anyhow!("Could not find cache dir"))?
        .join("better_tilbudsavis");
    std::fs::create_dir_all(path.clone())?;
    std::fs::write(
        path.join("offer_cache.json"),
        serde_json::to_string(offers).context("Failed to serialize offers to JSON")?,
    )
    .context("could not write offer cache")?;
    userdata.cache_updated();
    Ok(())
}

fn retrieve_cached_offers() -> Result<Vec<Offer>> {
    let path = dirs::cache_dir()
        .ok_or(anyhow!("Could not find cache dir"))?
        .join("better_tilbudsavis/offer_cache.json");
    let offer_cache_str = std::fs::read_to_string(path).context("Offer cache not found")?;
    serde_json::from_str(&offer_cache_str).context("Offer cache has invalid JSON")
}

async fn retrieve_offers_from_remote(userdata: &mut UserData) -> Vec<Offer> {
    futures::future::join_all(
        userdata
            .favorites
            .iter()
            .map(|dealer| dealer.remote_offers_for_dealer()),
    )
    .await
    .into_iter()
    .flatten()
    .collect()
}
