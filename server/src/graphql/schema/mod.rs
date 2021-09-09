use juniper::{EmptyMutation, EmptySubscription};

#[derive(GraphQLEnum)]
enum Episode {
    NewHope,
    Empire,
    Jedi,
}

#[derive(GraphQLObject)]
#[graphql(description="A humanoid creature in the Star Wars universe")]
struct Human {
    id: String,
    name: String,
    appears_in: Vec<Episode>,
    home_planet: String,
}

// To make our context usable by Juniper, we have to implement a marker trait.
impl juniper::Context for crate::MeitiDb {}

pub struct Query;

#[juniper::graphql_object(Context = crate::MeitiDb)]
impl Query {
    fn apiVersion() -> &'static str {
        "1.0"
    }
}

// A root schema consists of a query and a mutation.
// Request queries can be executed against a RootNode.
pub type Schema = juniper::RootNode<'static, Query, EmptyMutation<crate::MeitiDb>, EmptySubscription<crate::MeitiDb>>;
