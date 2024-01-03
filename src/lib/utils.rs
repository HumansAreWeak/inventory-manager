/**
 * This file is part of invman.
 *
 * invman - Manage your inventory easily, declaratively, without the headache.
 * Copyright (C) 2023  Maik Steiger <m.steiger@csurielektronics.com>
 *
 * invman is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * invman is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with invman. If not, see <https://www.gnu.org/licenses/>.
 */
use crate::common::args::{ColumnType, SchemaDeclaration};
use anyhow::{anyhow, bail, Result};

pub trait SchemaDeclarationVerify {
    /**
     * Check if a given String is in schema notation and is found within the vector
     * of declarations. Then the string's value is checked against that schema.
     *
     * @returns A tuple in (name, value) syntax
     */
    fn check_against_declaration(
        &self,
        declarations: &Vec<SchemaDeclaration>,
    ) -> Result<(String, String)>;
}

impl SchemaDeclarationVerify for String {
    fn check_against_declaration(
        &self,
        declarations: &Vec<SchemaDeclaration>,
    ) -> Result<(String, String)> {
        let schema_not = self.split_once("=");
        if schema_not.is_none() {
            bail!("Given string {} is not in valid schema notation", self);
        }
        let (name, value) = schema_not.unwrap();
        let name = String::from(name);
        let value = String::from(value);
        let schema = declarations.iter().find(|e| e.name == name);
        if schema.is_none() {
            bail!("Field {} could not be found in schema declaration", name);
        }

        let schema = schema.unwrap();
        return match schema.column_type {
            ColumnType::BOOL => {
                if value.to_ascii_lowercase() == "true" {
                    Ok((name, String::from("true")))
                } else if value.to_ascii_lowercase() == "false" {
                    Ok((name, String::from("false")))
                } else {
                    Err(anyhow!("Value not of boolean type"))
                }
            }
            ColumnType::VARCHAR | ColumnType::TEXT => {
                let value = String::from(value);
                let value_len = u32::try_from(value.len()).unwrap();
                if value_len < schema.min_length {
                    Err(anyhow!(
                        "Field's {} length is less than schema's min length",
                        name
                    ))
                } else if value_len > schema.max_length {
                    Err(anyhow!(
                        "Field's {} length is more than schema's max length",
                        name
                    ))
                } else {
                    let value = format!("\"{}\"", value);
                    Ok((name, value))
                }
            }
            ColumnType::INT => match value.parse::<i64>() {
                Ok(s) => {
                    if schema.min > 0 && s < schema.min.into() {
                        bail!("Field {} is smaller than schema's min", name);
                    } else if schema.max > 0 && s > schema.max.into() {
                        bail!("Field {} is larger than schema's max", name);
                    } else {
                        Ok((name, value))
                    }
                }
                Err(_) => Err(anyhow!("Field {} is not a valid integer type", name)),
            },
            ColumnType::REAL => match value.parse::<f64>() {
                Ok(s) => {
                    if s < schema.min.into() {
                        bail!("Field {} is smaller than schema's min", name);
                    } else if s > schema.max.into() {
                        bail!("Field {} is larger than schema's max", name);
                    } else {
                        Ok((name, value))
                    }
                }
                Err(_) => Err(anyhow!("Field {} is not a valid real type", name)),
            },
        };
    }
}

pub trait InvManSerialization {
    fn to_json(&self) -> String;
}

pub trait InvManDbHelper {
    fn to_sql_names(&self) -> String;
}

impl InvManSerialization for Vec<SchemaDeclaration> {
    fn to_json(&self) -> String {
        let mut jsons = self
            .iter()
            .map(|e| e.to_json())
            .collect::<Vec<String>>()
            .join(",");
        jsons.insert(0, '[');
        jsons.push(']');
        return jsons;
    }
}

impl InvManDbHelper for Vec<SchemaDeclaration> {
    fn to_sql_names(&self) -> String {
        self.iter()
            .map(|e| e.name.to_owned())
            .collect::<Vec<String>>()
            .join(",")
    }
}
