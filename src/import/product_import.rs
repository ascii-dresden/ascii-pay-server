use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::path::PathBuf;

use crate::{
    database::DatabaseConnection,
    error::ServiceResult,
    models::{CoinAmount, CoinType, Image, Product},
};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
struct OldCategory {
    pub name: String,
}
impl Default for OldCategory {
    fn default() -> Self {
        Self {
            name: "".to_owned(),
        }
    }
}

type Money = i32;
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum OldStampType {
    None,
    Coffee,
    Bottle,
}
impl Default for OldStampType {
    fn default() -> Self {
        Self::None
    }
}

/// Represent a product
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OldProduct {
    pub id: String,
    pub name: String,
    pub price: Money,
    #[serde(default)]
    pub pay_with_stamps: OldStampType,
    #[serde(default)]
    pub give_stamps: OldStampType,
    pub nickname: Option<String>,
    pub image: Option<String>,
    pub barcode: Option<String>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub category: OldCategory,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
struct OldCategoryFile {
    #[serde(flatten)]
    pub category: OldCategory,
    pub products: Vec<OldProduct>,
}

pub async fn load_product_repo_into_database(
    db: &mut DatabaseConnection,
    path: &str,
) -> ServiceResult<HashMap<String, u64>> {
    let path = PathBuf::from(path);
    let config_file = path.join("main.json");

    let mut file = File::open(&config_file).unwrap();
    let dataset: Vec<OldCategoryFile> = serde_json::from_reader(&mut file).unwrap();
    let mut dict = HashMap::<String, u64>::new();

    for category in dataset {
        for mut product in category.products {
            product.category = category.category.clone();

            let mut price = HashMap::<CoinType, i32>::new();
            price.insert(CoinType::Cent, product.price);
            match product.pay_with_stamps {
                OldStampType::Coffee => {
                    price.insert(CoinType::CoffeeStamp, 10);
                }
                OldStampType::Bottle => {
                    price.insert(CoinType::BottleStamp, 10);
                }
                _ => {}
            };

            let mut bonus = HashMap::<CoinType, i32>::new();
            match product.give_stamps {
                OldStampType::Coffee => {
                    bonus.insert(CoinType::CoffeeStamp, 1);
                }
                OldStampType::Bottle => {
                    bonus.insert(CoinType::BottleStamp, 1);
                }
                _ => {}
            };

            let new_product = db
                .store_product(Product {
                    id: 0,
                    name: product.name,
                    price: CoinAmount(price),
                    bonus: CoinAmount(bonus),
                    nickname: product.nickname,
                    image: None,
                    barcode: product.barcode,
                    category: product.category.name,
                    tags: product.flags,
                })
                .await?;

            dict.insert(product.id, new_product.id);

            if let Some(image) = product.image {
                let image_path = path.join(image);
                let bytes = std::fs::read(image_path).unwrap();

                db.store_product_image(
                    new_product.id,
                    Image {
                        mimetype: "image/png".to_owned(),
                        data: bytes,
                    },
                )
                .await?;
            }
        }
    }

    Ok(dict)
}
