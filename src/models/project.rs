use serde::{de, Deserialize, Deserializer, Serialize};
use sqlx::FromRow;
use std::{fmt, marker::PhantomData};
use uuid::Uuid;

#[derive(Default)]
pub enum Patch<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<'de, T> Deserialize<'de> for Patch<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PatchVisitor<T>(PhantomData<T>);

        impl<'de, T> de::Visitor<'de> for PatchVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = Patch<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("null or a value")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Patch::Null)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                T::deserialize(deserializer).map(Patch::Value)
            }
        }

        deserializer.deserialize_option(PatchVisitor(PhantomData))
    }
}

#[derive(Serialize, FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
}
#[derive(Deserialize)]
pub struct ProjectInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct ProjectUpdate {
    #[serde(default)]
    pub name: Patch<String>,
    #[serde(default)]
    pub description: Patch<String>,
}

#[derive(Serialize, FromRow)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: i16,
    pub due_date: Option<String>,
}
#[derive(Deserialize)]
pub struct TaskInput {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<i16>,
    pub due_date: Option<String>,
}
#[derive(Deserialize)]
pub struct TaskUpdate {
    pub title: Option<String>,
    #[serde(default)]
    pub description: Patch<String>,
    pub status: Option<String>,
    pub priority: Option<i16>,
    #[serde(default)]
    pub due_date: Patch<String>,
}
