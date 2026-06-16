use quote::quote;
use syn::ext::IdentExt;
use syn::{DeriveInput, parse_macro_input};

/// Takes a struct which has FromRow derived, and creates crud functions for a DB.
/// For now it inserts all fields that are not id, and skips any fields with `#[core_db_skip_insert]`
#[proc_macro_derive(CoreDbCrud, attributes(core_db_skip_insert, core_db_table, core_db_id))]
pub fn db_crud(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;
    let attrs = input.attrs;
    let syn::Data::Struct(ds) = input.data else {
        panic!("CoreDbCrud can only be derived for structs");
    };
    let syn::Fields::Named(nf) = ds.fields else {
        panic!("CoreDbCrud requires named fields");
    };
    let Some(table) = get_table(&attrs) else {
        panic!("CoreDbCrud requires a table name");
    };
    let insert_fields = list_insert_fields(&nf);

    if insert_fields.is_empty() {
        panic!("CoreDbCrud can only be derived if there is at least one valid field");
    }
    let Some(id_field) = get_id_field(&nf) else {
        panic!(
            "CoreDbCrud has no `id` field or field marked with `#[core_db_id]` which acts as a surrogate"
        );
    };

    let cleaned_fields = insert_fields.iter().map(|x| x.unraw()).collect::<Vec<_>>();

    let mut q_marks: String = std::iter::repeat_n("$", cleaned_fields.len())
        .enumerate()
        .map(|(i, x)| format!("{x}{}, ", i + 1))
        .collect();
    let mut fields_string: String = cleaned_fields.iter().map(|x| format!("{x}, ")).collect();
    let mut update_field_string: String = cleaned_fields
        .iter()
        .enumerate()
        .map(|(i, x)| format!("{x} = ${}, ", i + 1))
        .collect();

    for x in [&mut q_marks, &mut fields_string, &mut update_field_string] {
        x.pop();
        x.pop();
    }

    let insert_str = format!("INSERT INTO {table}({fields_string}) values({q_marks}) RETURNING *");
    let get_by_id_str = format!("SELECT * FROM {table} WHERE {id_field} = $1");
    let update_str = format!(
        "UPDATE {table} SET {update_field_string} WHERE {id_field}=${}",
        cleaned_fields.len() + 1
    );
    let delete_str = format!("DELETE FROM {table} WHERE {id_field} = $1");

    let tokens = quote! {
        impl crate::core_schema::CoreDbCrud for #struct_name {
            /// Достать ключ `id` или псевдо-ключ
            fn pkey(&self) -> i64 {
                self.#id_field
            }
        }

        impl #struct_name {
            const TABLE: &str = #table;

            /// Вставить, вернуть его ид
            pub(crate) async fn insert<'a, T:  sqlx::PgExecutor<'a>>(&mut self, exc: T) -> crate::error::Result<i64> {
                let new_self = sqlx::query_as::<_, Self>(#insert_str)
                    #( .bind(&self.#insert_fields) )*
                    .fetch_one(exc)
                    .await?;
                *self = new_self;
                Ok(self.pkey())
            }

            pub async fn update<'a, T: sqlx::PgExecutor<'a>>(&self, exc: T) -> crate::error::Result<()> {
                sqlx::query(#update_str)
                    #( .bind(&self.#insert_fields) )*
                    .bind(self.pkey())
                    .execute(exc)
                    .await?;
                Ok(())
            }

            /// Достать. NB: Если используем `#[core_db_id]` и ключ не настоящий, то достаём только первый
            /// Случай. Фунцкии чтобы достать все надо делать в ручную (но они и не есть CRUD).
            pub async fn get_by_id<'a, T: sqlx::PgExecutor<'a>>(id: i64, exc: T) -> crate::error::Result<Self> {
                sqlx::query_as::<_, Self>(#get_by_id_str)
                    .bind(id)
                    .fetch_one(exc)
                    .await
                    .map_err(Into::into)
            }

            /// Удалить
            pub async fn delete_by_id<'a, T: sqlx::PgExecutor<'a>>(id: i64, exc: T) -> crate::error::Result<()> {
                sqlx::query(#delete_str)
                    .bind(id)
                    .execute(exc)
                    .await?;
                Ok(())
            }

            /// Удалить по другому
            pub async fn delete<'a, T: sqlx::PgExecutor<'a>>(&self, exc: T) -> crate::error::Result<()> {
                Self::delete_by_id(self.pkey(), exc).await
            }
        }
    };
    proc_macro::TokenStream::from(tokens)
}

fn get_table(attrs: &[syn::Attribute]) -> Option<String> {
    let attr = attrs
        .iter()
        .filter(|a| matches!(a.style, syn::AttrStyle::Outer))
        .filter_map(|a| match &a.meta {
            syn::Meta::NameValue(p) => Some(p),
            _ => None,
        })
        .find(|x| x.path.is_ident("core_db_table"))?;
    let lit = match &attr.value {
        syn::Expr::Lit(syn::ExprLit { lit, .. }) => lit,
        _ => return None,
    };
    match lit {
        syn::Lit::Str(s) => Some(s.value()),
        _ => None,
    }
}

/// If we have `#[core_db_skip_insert]`, mark for skipping
fn has_attr(attrs: &[syn::Attribute], attr_name: &str) -> bool {
    attrs
        .iter()
        .filter(|a| matches!(a.style, syn::AttrStyle::Outer))
        .any(|a| match &a.meta {
            syn::Meta::Path(p) => p.is_ident(attr_name),
            _ => false,
        })
}

/// If we have `#[core_db_skip_insert]`, mark for skipping
fn has_skip(attrs: &[syn::Attribute]) -> bool {
    has_attr(attrs, "core_db_skip_insert")
}

/// If we have `#[core_db_skip_insert]`, mark for skipping
fn has_key(attrs: &[syn::Attribute]) -> bool {
    has_attr(attrs, "core_db_id")
}

fn list_insert_fields(fields: &syn::FieldsNamed) -> Vec<syn::Ident> {
    fields
        .named
        .iter()
        .filter(|f| !has_skip(&f.attrs))
        .filter_map(|f| f.ident.clone())
        .collect::<Vec<_>>()
}

fn get_id_field(fields: &syn::FieldsNamed) -> Option<syn::Ident> {
    fields
        .named
        .iter()
        .find(|f| {
            has_key(&f.attrs)
                || f.ident
                    .as_ref()
                    .map(|x| &x.to_string() == "id")
                    .unwrap_or(false)
        })
        .and_then(|f| f.ident.clone())
}
