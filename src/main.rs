extern crate kuchiki;

use html5ever::{interface::QualName, local_name, namespace_url, ns};
use kuchiki::{traits::*, NodeRef};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub html_input_file: String,
    pub output_dir: String,
    pub html_output_file: String,
}

fn main() {
    // Settings.
    const CONFIG_FILE: &str = "cssinliner.config.json";
    let settings = match get_settings(CONFIG_FILE) {
        None => {
            eprintln!("Could not read the configuration file {}", CONFIG_FILE);
            return;
        }
        Some(s) => s,
    };

    // Path variables based on the settings.
    let html_input_file = Path::new(&settings.html_input_file).to_path_buf();
    let html_output_file =
        Path::new(&settings.output_dir).join(Path::new(&settings.html_output_file).to_path_buf());

    // Cleanup: Remove the output html file, if it is existing.
    if html_output_file.is_file() && !html_input_file.eq(&html_output_file) {
        match fs::remove_file(&html_output_file) {
            Err(err) => {
                eprintln!(
                    "Could not remove the existing file {:?} Error:{}",
                    html_output_file, err
                );
                return;
            }
            Ok(_) => {}
        }
    }

    // Read and parse the input html file.
    let document_result = kuchiki::parse_html()
        .from_utf8()
        .from_file(&html_input_file);

    let document = match document_result {
        Err(err) => {
            eprintln!(
                "Error occured while reading the input html file {:?} Error: {}",
                html_input_file, err
            );
            return;
        }
        Ok(d) => d,
    };

    // Read external css files and insert contents of them as inline css styles into the html.
    const LINK_TAG_SELECTOR: &str = r#"link[rel="stylesheet"]:not([href*="?external"])"#;

    // let link_tag_matches = document.select(LINK_TAG_SELECTOR).unwrap();
    let link_tag_matches;
    match document.select(LINK_TAG_SELECTOR) {
        Err(err) => {
            eprintln!(
                "Could not get link tags from the html file {:?} Error: {:?}",
                html_input_file, err
            );
            return;
        }
        Ok(l) => link_tag_matches = l,
    }

    // Note: Have to traverse in the reverse order.
    //       Otherwise, adding/removing of multiple html elements are not working as expected.
    for link_tag_match in link_tag_matches.rev() {
        // Read the attributes of the link tag.
        let attributes = link_tag_match.attributes.borrow();
        let rel_attr_val = match attributes.get("rel") {
            None => {
                eprintln!("Could not read the attribute 'rel'.");
                continue;
            }
            Some(v) => v,
        };
        let href_attr_val = match attributes.get("href") {
            None => {
                eprintln!("Could not read the attribute 'href'.");
                continue;
            }
            Some(v) => v,
        };

        // Need to consider only the link tags with non empty hrefs.
        if rel_attr_val == "stylesheet" && href_attr_val.trim() != "" {
            let new_style_node =
                NodeRef::new_element(QualName::new(None, ns!(html), local_name!("style")), None);

            // CSS File: Get the external css file name.
            let mut css_file_dir = PathBuf::from(&settings.html_input_file);
            css_file_dir.pop();
            let href_path = Path::new(&href_attr_val);
            let prefix_stripped_result = href_path.strip_prefix("/");
            let mut prefix_stripped_href_path = href_path;
            match prefix_stripped_result {
                Ok(prefix_stripped) => prefix_stripped_href_path = prefix_stripped,
                Err(_) => {}
            };

            let css_file = css_file_dir.join(prefix_stripped_href_path);
            if !css_file.is_file() {
                eprintln!("Could not find the CSS file: {:?}", css_file.to_str());
                continue;
            }

            // CSS File: Read the external css file contents.
            let css_file_content_result = fs::read_to_string(css_file);
            let css_file_content = match css_file_content_result {
                Ok(css_file_content) => css_file_content,
                Err(err) => {
                    eprintln!("Error while reading CSS file. Error: {}", err);
                    continue;
                }
            };

            // Insert contents of the css file as inline style.
            new_style_node.append(NodeRef::new_text(css_file_content));
            let link_node = link_tag_match.as_node();
            link_node.insert_before(new_style_node);

            // Remove the link node.
            link_node.detach(); // Note:  Have to traverse the for loop items in reverse order. Otherwise, this is not working as expected.
        }
    }

    // Output: Directory of the output html file.
    let html_output_dir = match html_output_file.parent() {
        None => {
            eprintln!("Could not get the directory for saving the output html file.");
            return;
        }
        Some(d) => d,
    };

    // Output: Create the ouput folder, if it is not already existing.
    if !html_output_dir.is_dir() {
        match fs::create_dir_all(&html_output_dir) {
            Err(err) => {
                eprintln!(
                    "Could not create the directory for saving the output html file. Error: {}",
                    err
                );
                return;
            }
            Ok(_) => {}
        }
    }

    // Output: Write the output html file.
    match document.serialize_to_file(html_output_file) {
        Err(err) => {
            eprintln!("Could not write the output html file. Error: {}", err);
            return;
        }
        Ok(_) => {}
    }
}

fn get_settings(json_file: &str) -> Option<Settings> {
    // Read the contents of the json file.
    let data_result = fs::read_to_string(json_file);
    let data: String;
    match data_result {
        Err(err) => {
            eprintln!("Could not read the file: {} Error: {}", &json_file, err);
            return None;
        }
        Ok(d) => {
            data = d;
        }
    }

    // Parse the json string.
    let json_result = serde_json::from_str(&data);
    let json: serde_json::Value;
    match json_result {
        Err(err) => {
            eprintln!(
                "JSON file does not have correct format: {} Error: {}",
                &json_file, err
            );
            return None;
        }
        Ok(j) => {
            json = j;
        }
    }

    let hif = get_json_str_val(&json, "htmlInputFile")?;
    let od = get_json_str_val(&json, "outputDir")?;
    let hof = get_json_str_val(&json, "htmlOutputFile")?;

    let settings = Settings {
        html_input_file: String::from(hif),
        output_dir: String::from(od),
        html_output_file: String::from(hof),
    };

    return Some(settings);
}

fn get_json_str_val<'a>(json: &'a serde_json::Value, field_name: &str) -> Option<&'a str> {
    match json.get(field_name) {
        Some(val) => val.as_str(),
        None => {
            println!(
                "Could not get value of the field {} from json file",
                field_name
            );
            return None;
        }
    }
}
