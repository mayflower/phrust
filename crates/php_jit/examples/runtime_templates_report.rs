use php_jit::region_ir::templates::lower_default_template_catalog;

fn main() {
    let json = std::env::args().skip(1).any(|arg| arg == "--json");
    let report = lower_default_template_catalog();
    if json {
        print!("{}", report.to_json());
    } else {
        print!("{}", report.to_markdown());
    }
}
