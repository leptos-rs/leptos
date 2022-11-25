// use crate::IntoNode;

// #[derive(typed_builder::TypedBuilder)]
// struct EachProps {}

// #[allow(non_snake_case)]
// /// ```html
// /// <ul>
// ///     <!-- <Each> -->
// ///     <!-- <Item> -->
// ///     <li>1</li>
// ///     <!-- </Item> -->
// ///     <!-- <Item> -->
// ///     <li>2</li>
// ///     <!-- </Item> -->
// ///     <!-- </Each> -->
// /// </ul>
// /// ```
// struct Each<I, T, IF>
// where
//     for<'a> &'a I: IntoIterator<Item = T>,
//     T: Eq,
//     IF: Fn(&T) ->
// {
//     items: I,
// }
