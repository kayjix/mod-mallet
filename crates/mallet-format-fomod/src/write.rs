//! Writes FOMOD installers, for the patches Mod Mallet makes (ADR 0053).
//!
//! Only what a patch needs is written: one page with one group of options,
//! any number of which may be chosen. Each option installs files and is
//! ticked in advance when the plugins it needs are active, and greyed out
//! when they are not.

use std::fmt::Write;

/// An installer of one page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installer {
    /// The name of what is installed, as the mod manager shows it.
    pub name: String,
    /// Who made it, if anybody says.
    pub author: Option<String>,
    /// Its version, if it has one.
    pub version: Option<String>,
    /// A few words about it.
    pub description: String,
    /// The title of the page.
    pub page: String,
    /// The title of the group of options on the page.
    pub group: String,
    /// The options, in the order they are shown.
    pub options: Vec<Choice>,
}

/// One option of an [`Installer`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// Its title.
    pub name: String,
    /// What it does.
    pub description: String,
    /// The files it installs: the path in the archive, and the path below
    /// the data directory.
    pub files: Vec<(String, String)>,
    /// The plugins that must be active for it to apply. Without any, it is
    /// always ticked in advance.
    pub needs: Vec<String>,
}

/// The text of `fomod/ModuleConfig.xml` for `installer`.
pub fn module_config(installer: &Installer) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <config xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:noNamespaceSchemaLocation=\"http://qconsulting.ca/fo3/ModConfig5.0.xsd\">\n",
    );
    let _ = writeln!(
        xml,
        "  <moduleName>{}</moduleName>",
        escape(&installer.name)
    );
    xml.push_str("  <installSteps order=\"Explicit\">\n");
    let _ = writeln!(
        xml,
        "    <installStep name=\"{}\">",
        escape(&installer.page)
    );
    xml.push_str("      <optionalFileGroups order=\"Explicit\">\n");
    let _ = writeln!(
        xml,
        "        <group name=\"{}\" type=\"SelectAny\">",
        escape(&installer.group)
    );
    xml.push_str("          <plugins order=\"Explicit\">\n");
    for choice in &installer.options {
        write_choice(&mut xml, choice);
    }
    xml.push_str(
        "          </plugins>\n        </group>\n      </optionalFileGroups>\n    \
         </installStep>\n  </installSteps>\n</config>\n",
    );
    xml
}

fn write_choice(xml: &mut String, choice: &Choice) {
    let indent = "            ";
    let _ = writeln!(xml, "{indent}<plugin name=\"{}\">", escape(&choice.name));
    let _ = writeln!(
        xml,
        "{indent}  <description>{}</description>",
        escape(&choice.description)
    );
    let _ = writeln!(xml, "{indent}  <files>");
    for (source, destination) in &choice.files {
        let _ = writeln!(
            xml,
            "{indent}    <file source=\"{}\" destination=\"{}\" priority=\"0\"/>",
            escape(source),
            escape(destination)
        );
    }
    let _ = writeln!(xml, "{indent}  </files>");
    let _ = writeln!(xml, "{indent}  <typeDescriptor>");
    if choice.needs.is_empty() {
        let _ = writeln!(xml, "{indent}    <type name=\"Recommended\"/>");
    } else {
        let _ = writeln!(xml, "{indent}    <dependencyType>");
        let _ = writeln!(xml, "{indent}      <defaultType name=\"NotUsable\"/>");
        let _ = writeln!(xml, "{indent}      <patterns>");
        let _ = writeln!(xml, "{indent}        <pattern>");
        let _ = writeln!(xml, "{indent}          <dependencies operator=\"And\">");
        for plugin in &choice.needs {
            let _ = writeln!(
                xml,
                "{indent}            <fileDependency file=\"{}\" state=\"Active\"/>",
                escape(plugin)
            );
        }
        let _ = writeln!(xml, "{indent}          </dependencies>");
        let _ = writeln!(xml, "{indent}          <type name=\"Recommended\"/>");
        let _ = writeln!(xml, "{indent}        </pattern>");
        let _ = writeln!(xml, "{indent}      </patterns>");
        let _ = writeln!(xml, "{indent}    </dependencyType>");
    }
    let _ = writeln!(xml, "{indent}  </typeDescriptor>");
    let _ = writeln!(xml, "{indent}</plugin>");
}

/// The text of `fomod/info.xml` for `installer`: its name, author, version
/// and description, which mod managers show for the installed mod.
pub fn info(installer: &Installer) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<fomod>\n");
    let _ = writeln!(xml, "  <Name>{}</Name>", escape(&installer.name));
    if let Some(author) = &installer.author {
        let _ = writeln!(xml, "  <Author>{}</Author>", escape(author));
    }
    if let Some(version) = &installer.version {
        let _ = writeln!(xml, "  <Version>{}</Version>", escape(version));
    }
    let _ = writeln!(
        xml,
        "  <Description>{}</Description>",
        escape(&installer.description)
    );
    xml.push_str("</fomod>\n");
    xml
}

/// `text` as XML text or attribute value. Characters XML cannot hold are
/// left out.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\t' | '\n' | '\r' => escaped.push(' '),
            c if c.is_control() || matches!(c, '\u{FFFE}' | '\u{FFFF}') => {}
            c => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::installer::{Condition, FileState, OptionStatus, Selection};
    use proptest::prelude::*;

    use super::*;

    fn installer(options: Vec<Choice>) -> Installer {
        Installer {
            name: "Mod Mallet Patch".to_owned(),
            author: Some("Someone <&> \"quoted\"".to_owned()),
            version: Some("1.0".to_owned()),
            description: "Hides objects.".to_owned(),
            page: "Parts".to_owned(),
            group: "Mods".to_owned(),
            options,
        }
    }

    #[test]
    fn a_written_installer_is_read_back() {
        let xml = module_config(&installer(vec![
            Choice {
                name: "Town.esp against Road's.esp".to_owned(),
                description: "12 objects".to_owned(),
                files: vec![("parts/1_SWAP.ini".to_owned(), "1_SWAP.ini".to_owned())],
                needs: vec!["Town.esp".to_owned(), "Road's.esp".to_owned()],
            },
            Choice {
                name: "Town.esp".to_owned(),
                description: "a sign".to_owned(),
                files: vec![("parts/2_SWAP.ini".to_owned(), "2_SWAP.ini".to_owned())],
                needs: Vec::new(),
            },
        ]));
        let steps = crate::parse(&xml).unwrap();
        assert_eq!(steps.len(), 1);
        let group = &steps[0].groups[0];
        assert_eq!(group.selection, Selection::Any);
        let [first, second] = group.options.as_slice() else {
            panic!("two options expected");
        };
        assert_eq!(first.name, "Town.esp against Road's.esp");
        assert_eq!(first.default_status, OptionStatus::NotUsable);
        assert_eq!(
            first.rules,
            [(
                Condition::All(vec![
                    Condition::File {
                        file: "Town.esp".to_owned(),
                        state: FileState::Active,
                    },
                    Condition::File {
                        file: "Road's.esp".to_owned(),
                        state: FileState::Active,
                    },
                ]),
                OptionStatus::Recommended,
            )]
        );
        assert_eq!(second.default_status, OptionStatus::Recommended);
        assert_eq!(second.rules, []);
    }

    #[test]
    fn info_names_the_author() {
        let xml = info(&installer(Vec::new()));
        let document = roxmltree::Document::parse(&xml).unwrap();
        let author = document
            .descendants()
            .find(|node| node.has_tag_name("Author"))
            .and_then(|node| node.text());
        assert_eq!(author, Some("Someone <&> \"quoted\""));
    }

    proptest! {
        #[test]
        fn any_names_make_well_formed_xml(name in ".{0,40}", plugin in ".{0,40}") {
            let xml = module_config(&installer(vec![Choice {
                name: name.clone(),
                description: name.clone(),
                files: vec![(name.clone(), plugin.clone())],
                needs: vec![plugin],
            }]));
            prop_assert!(roxmltree::Document::parse(&xml).is_ok());
            prop_assert!(roxmltree::Document::parse(&info(&installer(Vec::new()))).is_ok());
        }
    }
}
