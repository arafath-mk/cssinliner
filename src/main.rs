extern crate kuchiki;

use html5ever::{interface::QualName, local_name, namespace_url, ns};
use kuchiki::{traits::*, NodeRef};
use std::fs;
use std::path::Path;

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub html_input_file: String,
    pub output_dir: String,
    pub html_output_file: String,
}

fn main() {
    // Settings.
    let settings = Settings {
        html_input_file: String::from("input.html"),
        output_dir: String::from("dist/"),
        html_output_file: String::from("minified/index.html"),
    };

    // Path variables based on the settings.
    let html_input_file = Path::new(&settings.html_input_file).to_path_buf();
    let html_output_file =
        Path::new(&settings.output_dir).join(Path::new(&settings.html_output_file).to_path_buf());

    // Cleanup: Remove the output html file, if it is existing.
    if html_output_file.is_file() {
        fs::remove_file(&html_output_file).unwrap();
    }

    // Read and parse the input html file.
    let document = kuchiki::parse_html()
        .from_utf8()
        .from_file(&html_input_file)
        .unwrap();

    // Read external css files and insert contents of them as inline css styles into the html.
    const LINK_TAG_SELECTOR: &str = r#"link[rel="stylesheet"]:not([href*="?external"])"#;

    let link_tag_matches = document.select(LINK_TAG_SELECTOR).unwrap();
    // Note: Have to traverse in the reverse order.
    //       Otherwise, adding/removing of multiple html elements are not working as expected.
    for link_tag_match in link_tag_matches.rev() {
        let link_node = link_tag_match.as_node();

        // Read the attributes of the link tag.
        let attributes = link_tag_match.attributes.borrow();
        let rel_attr_val = attributes.get("rel").unwrap();
        let href_attr_val = attributes.get("href").unwrap();

        // Need to consider only the link tags with non empty hrefs.
        if rel_attr_val == "stylesheet" && href_attr_val.trim() != "" {
            let new_style_node =
                NodeRef::new_element(QualName::new(None, ns!(html), local_name!("style")), None);

            new_style_node.append(NodeRef::new_text(".b{color:blue}"));
            link_node.insert_before(new_style_node);
            link_node.detach(); // Note:  Have to traverse the for loop items in reverse order. Otherwise, this is not working as expected.
        }
    }

    // Output: Directory of the output html file.
    let html_output_dir = html_output_file.parent().unwrap().to_path_buf();

    // Output: Create the ouput folder, if it is not already existing.
    if !html_output_dir.is_dir() {
        fs::create_dir_all(&html_output_dir).unwrap();
    }

    // Output: Write the output html file.
    document.serialize_to_file(html_output_file).unwrap();
}
