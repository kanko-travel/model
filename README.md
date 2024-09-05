# Model
A simple ORM for use with sqlx and postgres

[![Build status](https://badge.buildkite.com/a26e0920c7b5f082d47fd89e2b48860f18b7a7afec64acbad7.svg)](https://buildkite.com/kanko-travel/kanko-model-crate)

## Example Usage
### The Bakery Object Model

```rust
use model::{Model, Enum, Related, RelationDef};

#[derive(Clone, sqlx::FromRow, Model)]
#[model(table_name = "bakery", has_relations)]
struct Bakery {
  #[model(id)]
  id: Uuid,
  #[model(primary_key)]
  name: String
}

#[derive(Clone, sqlx::FromRow, Model)]
#[model(table_name = "cake", has_relations)]
struct Cake {
  #[model(id)]
  id: Uuid,
  #[model(unique)]
  bakery_id: Uuid,
  #[model(enum, primary_key)]
  cake_type: CakeType
  #[model(primary_key)]
  name: String
}

#[derive(Clone, Enum)]
enum CakeType {
  ButterCake,
  SpongeCake,
}

#[derive(Clone, sqlx::FromRow, Model)]
#[model(table_name = "topping", has_relations)]
struct Topping {
  #[model(id, primary_key)]
  id: Uuid,
  name: String
}
```

### Defining Relations for the Bakery Object Model
```rust
// ...continued from above

impl Related for Bakery {
  fn relation_definitions() -> Vec<RelationDef> {
    vec![
      Self::has_many::<Cake>("cakes", "bakery_id")
    ]
  }
}

impl Related for Cake {
  fn relation_definitions() -> Vec<RelationDef> {
    vec![
      Self::belongs_to::<Bakery>("bakery", "bakery_id"),
      Self::has_many_via::<Topping>("toppings", "cake_topping"),
    ]
  }
}

impl Related for Topping {
  fn relation_definitions() -> Vec<RelationDef> {
    vec![
      Self::has_many_via::<Cake>("cakes", "cake_topping"),
    ]
  }
}
```

### Performing CRUD Operations on the Bakery Object Model
```rust
// ...continued from above

let pool = sqlx::PgPool::connect("db_uri").await.unwrap();
let mut conn = pool.acquire().await.unwrap();

let bakery = Bakery {
  id: Uuid::new_v4(),
  name: "Awesome Bakery".into()
};


bakery
  .create()
  .execute(&mut conn)
  .await
  .unwrap();

let mut another_bakery_with_same_primary_key = Bakery {
  id: Uuid::new_v4(),
  name: "Awesome Bakery".into()
};

assert_neq!(bakery.id, another_bakery_with_same_primary_key.id);

another_bakery_with_same_primary_key
  .upsert()
  .execute(&mut conn)
  .await
  .unwrap()

assert_eq!(bakery.id, another_bakery_with_same_primary_key.id);

let mut cake = Cake {
  id: Uuid::new_v4(),
  bakery_id: bakery.id.clone(),
  cake_type: CakeType::ButterCake,
  name: "Southern Coconut Cake".into(),
};

cake
  .create()
  .execute(&mut conn)
  .await
  .unwrap()

cake.name = "Western Coconut Cake".into();

cake
  .update(&mut conn)
  .await
  .unwrap();

let topping = Topping {
  id: Uuid::new_v4(),
  name: "Coconut Flakes"
}

topping
  .create()
  .execute(&mut conn)
  .await
  .unwrap();

cake
  .create_association("toppings", topping.id.clone())
  .execute(&mut conn)
  .await
  .unwrap()

let butter_cakes_with_coconut_flakes = Cake::select()
  .filter(
    model::Filter::new()
      .field("cake_type").eq(CakeType::ButterCake)
      .and()
      .field("toppings.name").eq("Coconut Flakes".to_string())
  )
  .fetch_page(&mut conn)
  .await
  .unwrap();

for cake in butter_cakes_with_coconut_flakes.nodes {
  println!("{}", cake.name);
}

Cake::delete(&cake.id)
  .execute(&mut conn)
  .await
  .unwrap();

Cake::delete(&cake.id)
  .idempotent()
  .execute(&mut conn)
  .await
  .unwrap();
```
