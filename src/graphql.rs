use juniper::EmptyMutation, EmptySubscription, RootNode};

struct QueryRoot;
struct MutationRoot;

type Schema = RootNode<'static, QueryRoot, EmptyMutation, EmptySubscription>;
