use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

// use git2::Repository;
use log::error;

use super::StampType;
use crate::utils::{env, Money, ServiceError, ServiceResult};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone, SimpleObject, Default)]
pub struct Category {
    pub name: String,
}

/// Represent a product
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone, SimpleObject)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone)]
struct DatasetPath {
    config_file: PathBuf,
}

impl DatasetPath {
    pub fn new() -> Option<Self> {
        let path = PathBuf::from(env::PRODUCT_STORAGE.as_str());

        if path.exists() {
            if path.is_file() {
                return Some(Self { config_file: path });
            } else {
                return Some(Self {
                    config_file: path.join("main.json"),
                });
            }
        }

        None
    }

    pub fn read_dataset(&self) -> ServiceResult<HashMap<String, Product>> {
        let mut map = HashMap::new();

        let mut file = File::open(&self.config_file)?;
        let dataset: Vec<CategoryFile> = serde_json::from_reader(&mut file)?;

        for category in dataset {
            for mut product in category.products {
                product.category = category.category.clone();
                map.insert(product.id.clone(), product);
            }
        }

        Ok(map)
    }

    pub fn update_repo(&self) -> ServiceResult<bool> {
        let repo_path = self.config_file.parent();

        if let Some(_repo_path) = repo_path {
            // let _repo = Repository::open(repo_path)?;

            error!("TODO: Update of product repo is not supported!");
        }

        Ok(false)
    }
}

impl Product {
    pub fn update_dataset_repo() -> ServiceResult<()> {
        let dataset = DatasetPath::new();

        match dataset {
            Some(dataset) => {
                dataset.update_repo()?;
            }
            None => {
                error!("Product storage does not exists!");
            }
        }

        Ok(())
    }

    pub fn load_dataset() -> ServiceResult<()> {
        let dataset = DatasetPath::new();

        match dataset {
            Some(dataset) => {
                let map = dataset.read_dataset()?;

                let mut w = PRODUCT_DATASET.write()?;
                *w = map;
            }
            None => {
                error!("Product storage does not exists!");
            }
        }

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
