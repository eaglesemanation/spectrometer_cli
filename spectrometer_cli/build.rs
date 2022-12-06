extern crate embed_resource;

fn add_icon() {
    embed_resource::compile("resources.rc");
}

fn main() {
    add_icon()
}
