pub mod node;

mod test {
 #[test]
 fn start_node() {
  let mut node = super::node::start_node().unwrap();
  node.init();
 }
}
