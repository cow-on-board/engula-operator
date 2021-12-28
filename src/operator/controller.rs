use std::{fmt::Debug, hash::Hash};

use kube::api::Resource;
use kube::runtime::controller::Controller;

// pub struct Manager<'a, K>
// where
//     K: Clone + Resource + Debug + 'static,
//     K::DynamicType: Eq + Hash,
// {
//     runnables: &'a [Controller<K>],
// }

// impl<'a, K> Manager<'a, K>
// where
//     K: Clone + Resource + Debug + 'static,
//     K::DynamicType: Eq + Hash,
// {

// }
