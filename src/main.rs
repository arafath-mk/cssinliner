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
    const CONFIG_FILE: &str = "cssinliner.json";
    let settings_result = get_settings(CONFIG_FILE);
    let settings: Settings;
    match settings_result {
        None => {
            eprintln!("Could not read the configuration file {}", CONFIG_FILE);
            return;
        }
        Some(s) => settings = s,
    }

    // Path variables based on the settings.
    let html_input_file = Path::new(&settings.html_input_file).to_path_buf();
    let html_output_file =
        Path::new(&settings.output_dir).join(Path::new(&settings.html_output_file).to_path_buf());

    // Cleanup: Remove the output html file, if it is existing.
    if html_output_file.is_file() {
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

    let document: NodeRef;
    match document_result {
        Err(err) => {
            eprintln!(
                "Error occured while reading an input html file {:?} Error: {}",
                html_input_file, err
            );
            return;
        }
        Ok(d) => document = d,
    }

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

            new_style_node.append(NodeRef::new_text(css_file_content));
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
    let hif_opt = json.get(field_name);
    match hif_opt {
        Some(hif_val) => hif_val.as_str(),
        None => {
            println!(
                "Could not get value of the field {} from json file",
                field_name
            );
            return None;
        }
    }
}
