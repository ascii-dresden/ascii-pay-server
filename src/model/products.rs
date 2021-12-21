use std::{collections::HashMap, fs::File, hash::Hash, path::Path, sync::RwLock};

use log::error;

use super::StampType;
use crate::utils::{env, Money, ServiceError, ServiceResult};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone, SimpleObject, Default)]
pub struct Category {
    pub name: String,
}

/// Represent a product
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone, SimpleObject)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub price: Money,
    #[serde(default)]
    pub pay_with_stamps: StampType,
    #[serde(default)]
    pub give_stamps: StampType,
    pub nickname: Option<String>,
    pub image: Option<String>,
    pub barcode: Option<String>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub category: Category,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct CategoryFile {
    #[serde(flatten)]
    pub category: Category,
    pub products: Vec<Product>,
}

lazy_static::lazy_static! {
    static ref PRODUCT_DATASET: RwLock<HashMap<String, Product>> = RwLock::new(HashMap::new());
}

impl Product {
    pub fn load_dataset() -> ServiceResult<()> {
        let mut map = HashMap::new();
        if Path::new(env::PRODUCT_STORAGE.as_str()).exists() {
            let mut file = File::open(env::PRODUCT_STORAGE.as_str())?;
            let dataset: Vec<CategoryFile> = serde_json::from_reader(&mut file)?;

            for category in dataset {
                for mut product in category.products {
                    product.category = category.category.clone();

                    map.insert(product.id.clone(), product);
                }
            }
        } else {
            error!("Product storage does not exists!");
        }
        let mut w = PRODUCT_DATASET.write()?;
        *w = map;

        Ok(())
    }

    pub fn clone_for_output(&self) -> Self {
        let mut p = self.clone();

        if p.image.is_some() {
            p.image = Some(format!("/api/v1/product/{}/image", p.id));
        }

        p
    }

    pub fn get_image(id: &str) -> ServiceResult<String> {
        let filename = if let Some(p) = PRODUCT_DATASET.read()?.get(id) {
            p.image.clone()
        } else {
            return Err(ServiceError::NotFound);
        };

        if let Some(filename) = filename.as_deref() {
            let file = Path::new(env::PRODUCT_STORAGE.as_str())
                .parent()
                .unwrap_or_else(|| Path::new(env::PRODUCT_STORAGE.as_str()));
            let path_buf = file.join(filename);
            if Path::new(&path_buf).exists() {
                if let Some(p) = path_buf.to_str() {
                    return Ok(p.to_owned());
                }
            }
        }

        Err(ServiceError::NotFound)
    }

    /// List all products
    pub fn all() -> ServiceResult<Vec<Product>> {
        let results = PRODUCT_DATASET
            .read()?
            .iter()
            .map(|(_, p)| p.clone_for_output())
            .collect();
        Ok(results)
    }

    /// Get a product by the `id`
    pub fn get(id: &str) -> ServiceResult<Product> {
        if let Some(p) = PRODUCT_DATASET.read()?.get(id) {
            return Ok(p.clone_for_output());
        }

        Err(ServiceError::NotFound)
    }

    /// Get a product by the `barcode`
    pub fn get_by_barcode(barcode: &str) -> ServiceResult<Product> {
        let dataset = PRODUCT_DATASET.read()?;

        for (_, p) in dataset.iter() {
            if let Some(b) = p.barcode.as_deref() {
                if b == barcode {
                    return Ok(p.clone_for_output());
                }
            }
        }

        Err(ServiceError::NotFound)
    }
}
