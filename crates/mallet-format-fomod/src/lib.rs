//! Reads FOMOD installer descriptions.
//!
//! FOMOD is the installer format most mods for Bethesda's games use. The
//! archive of a mod contains a file `fomod/ModuleConfig.xml` that describes
//! the installer's pages:
//!
//! ```xml
//! <config>
//!   <installSteps>
//!     <installStep name="Patches">
//!       <optionalFileGroups>
//!         <group name="Other mods" type="SelectAny">
//!           <plugins>
//!             <plugin name="Patch for Some Mod">
//!               <typeDescriptor>
//!                 <dependencyType>
//!                   <defaultType name="Optional"/>
//!                   <patterns>
//!                     <pattern>
//!                       <dependencies>
//!                         <fileDependency file="SomeMod.esp" state="Active"/>
//!                       </dependencies>
//!                       <type name="Recommended"/>
//!                     </pattern>
//!                   </patterns>
//!                 </dependencyType>
//!               </typeDescriptor>
//!             </plugin>
//! ```
//!
//! [`parse`] turns that into the installer model of `mallet-core`, keeping
//! what decides which options are offered and recommended and leaving out
//! what is installed where. [`decode`] turns the bytes of the file into
//! text first: these files come in UTF-8 and, about as often, UTF-16.
//!
//! The format is read leniently. Mod authors write these files by hand and
//! with various tools, so anything unexpected is skipped instead of making
//! the whole installer unreadable (ADR 0013).

use mallet_core::facts::installer::{
    Condition, FileState, InstallerGroup, InstallerOption, InstallerStep, OptionStatus, Selection,
};
use roxmltree::{Document, Node};

/// The path of the installer description inside a mod archive, in lower
/// case. Archives spell it in every mix of cases, and often below one
/// top-level folder.
pub const MODULE_CONFIG_PATH: &str = "fomod/moduleconfig.xml";

/// Why an installer description could not be read at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FomodError {
    /// The file is not well-formed XML.
    #[error("the installer description is not valid XML: {0}")]
    InvalidXml(String),
}

/// Turns the bytes of an installer description into text.
///
/// UTF-16 is recognised by its byte order mark or, without one, by the NUL
/// bytes between ASCII characters. Everything else is read as UTF-8, with
/// invalid bytes replaced.
pub fn decode(bytes: &[u8]) -> String {
    let utf16 = |bytes: &[u8], decode: fn([u8; 2]) -> u16| {
        let (pairs, _odd_byte) = bytes.as_chunks::<2>();
        let units: Vec<u16> = pairs.iter().copied().map(decode).collect();
        String::from_utf16_lossy(&units)
    };

    match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        [first, 0, ..] if *first != 0 => utf16(bytes, u16::from_le_bytes),
        [0, second, ..] if *second != 0 => utf16(bytes, u16::from_be_bytes),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// Reads the steps of an installer from the text of its description.
///
/// An installer without steps, which installs the same files for everyone,
/// yields an empty list.
pub fn parse(xml: &str) -> Result<Vec<InstallerStep>, FomodError> {
    // The declared encoding describes the bytes the text was decoded from;
    // it says nothing about the text and would only confuse the parser.
    let document =
        Document::parse(xml).map_err(|error| FomodError::InvalidXml(error.to_string()))?;

    Ok(children(document.root_element(), "installSteps")
        .flat_map(|steps| ordered(steps, "installStep", step, |step| &step.name))
        .collect())
}

/// The child elements `name` of `node`, read with `read`, in the order
/// the installer shows them.
///
/// The `order` attribute of the list decides: `Explicit` keeps the order
/// of the file, `Descending` sorts by title from Z to A, and anything else,
/// including no attribute, sorts from A to Z as the format prescribes.
fn ordered<T>(
    node: Node<'_, '_>,
    name: &'static str,
    read: impl Fn(Node<'_, '_>) -> T,
    title: impl Fn(&T) -> &str,
) -> Vec<T> {
    let mut items: Vec<T> = children(node, name).map(read).collect();
    match node.attribute("order").unwrap_or_default() {
        "Explicit" => {}
        "Descending" => {
            items.sort_by_key(|item| std::cmp::Reverse(title(item).to_lowercase()));
        }
        _ => items.sort_by_key(|item| title(item).to_lowercase()),
    }
    items
}

fn step(node: Node<'_, '_>) -> InstallerStep {
    InstallerStep {
        name: attribute(node, "name"),
        visible: children(node, "visible").next().map(composite),
        groups: children(node, "optionalFileGroups")
            .flat_map(|groups| ordered(groups, "group", group, |group| &group.name))
            .collect(),
    }
}

fn group(node: Node<'_, '_>) -> InstallerGroup {
    let selection = match node.attribute("type").unwrap_or_default() {
        "SelectAtLeastOne" => Selection::AtLeastOne,
        "SelectAtMostOne" => Selection::AtMostOne,
        "SelectExactlyOne" => Selection::ExactlyOne,
        "SelectAll" => Selection::All,
        _ => Selection::Any,
    };
    InstallerGroup {
        name: attribute(node, "name"),
        selection,
        options: children(node, "plugins")
            .flat_map(|plugins| ordered(plugins, "plugin", option, |option| &option.name))
            .collect(),
    }
}

fn option(node: Node<'_, '_>) -> InstallerOption {
    let sets_flags = children(node, "conditionFlags")
        .flat_map(|flags| children(flags, "flag"))
        .map(|flag| {
            (
                attribute(flag, "name"),
                flag.text().unwrap_or_default().trim().to_owned(),
            )
        })
        .collect();

    let descriptor = children(node, "typeDescriptor").next();
    let fixed = descriptor.and_then(|descriptor| children(descriptor, "type").next());
    let dependent = descriptor.and_then(|descriptor| children(descriptor, "dependencyType").next());

    let (default_status, rules) = match (fixed, dependent) {
        (Some(fixed), _) => (status(fixed), Vec::new()),
        (None, Some(dependent)) => {
            let default_status = children(dependent, "defaultType")
                .next()
                .map_or(OptionStatus::Optional, status);
            let rules = children(dependent, "patterns")
                .flat_map(|patterns| children(patterns, "pattern"))
                .filter_map(|pattern| {
                    let condition = children(pattern, "dependencies").next().map(composite)?;
                    let status = children(pattern, "type").next().map(status)?;
                    Some((condition, status))
                })
                .collect();
            (default_status, rules)
        }
        (None, None) => (OptionStatus::Optional, Vec::new()),
    };

    InstallerOption {
        name: attribute(node, "name"),
        description: children(node, "description")
            .next()
            .and_then(|description| description.text())
            .unwrap_or_default()
            .trim()
            .to_owned(),
        sets_flags,
        default_status,
        rules,
    }
}

fn status(node: Node<'_, '_>) -> OptionStatus {
    match node.attribute("name").unwrap_or_default() {
        "Required" => OptionStatus::Required,
        "Recommended" => OptionStatus::Recommended,
        "CouldBeUsable" => OptionStatus::CouldBeUsable,
        "NotUsable" => OptionStatus::NotUsable,
        _ => OptionStatus::Optional,
    }
}

/// Reads an element that combines conditions: `dependencies`, `visible`
/// and nested `dependencies`.
fn composite(node: Node<'_, '_>) -> Condition {
    let parts: Vec<Condition> = node
        .children()
        .filter(Node::is_element)
        .map(|child| match child.tag_name().name() {
            "dependencies" => composite(child),
            "fileDependency" => Condition::File {
                file: attribute(child, "file"),
                state: match child.attribute("state").unwrap_or_default() {
                    "Inactive" => FileState::Inactive,
                    "Missing" => FileState::Missing,
                    _ => FileState::Active,
                },
            },
            "flagDependency" => Condition::Flag {
                name: attribute(child, "flag"),
                value: attribute(child, "value"),
            },
            // Minimum versions of the game or the mod manager and anything
            // newer than this reader.
            _ => Condition::Unknown,
        })
        .collect();

    if node.attribute("operator") == Some("Or") {
        Condition::Any(parts)
    } else {
        Condition::All(parts)
    }
}

fn children<'a, 'input>(
    node: Node<'a, 'input>,
    name: &'static str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children()
        .filter(move |child| child.is_element() && child.tag_name().name() == name)
}

fn attribute(node: Node<'_, '_>, name: &str) -> String {
    node.attribute(name).unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// An installer in the shape real ones have. Names are made up.
    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-16"?>
<config xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="http://qconsulting.ca/fo3/ModConfig5.0.xsd">
  <moduleName>Lighting Patches</moduleName>
  <requiredInstallFiles>
    <folder source="core" destination=""/>
  </requiredInstallFiles>
  <installSteps order="Explicit">
    <installStep name="Version">
      <optionalFileGroups order="Explicit">
        <group name="Brightness" type="SelectExactlyOne">
          <plugins order="Explicit">
            <plugin name="Default">
              <description>The author's vision.</description>
              <conditionFlags><flag name="brightness">default</flag></conditionFlags>
              <typeDescriptor><type name="Recommended"/></typeDescriptor>
            </plugin>
            <plugin name="Brighter">
              <description>For those who want to see.</description>
              <conditionFlags><flag name="brightness"> bright </flag></conditionFlags>
              <typeDescriptor><type name="Optional"/></typeDescriptor>
            </plugin>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
    <installStep name="Patches">
      <visible>
        <flagDependency flag="brightness" value="default"/>
      </visible>
      <optionalFileGroups order="Explicit">
        <group name="Cities" type="SelectAny">
          <plugins order="Explicit">
            <plugin name="City Overhaul">
              <description/>
              <files><file source="patches/city.esp" destination="city patch.esp"/></files>
              <typeDescriptor>
                <dependencyType>
                  <defaultType name="Optional"/>
                  <patterns>
                    <pattern>
                      <dependencies operator="And">
                        <fileDependency file="CityOverhaul.esp" state="Active"/>
                        <dependencies operator="Or">
                          <fileDependency file="Old.esp" state="Missing"/>
                          <gameDependency version="1.6"/>
                        </dependencies>
                      </dependencies>
                      <type name="Recommended"/>
                    </pattern>
                    <pattern>
                      <dependencies>
                        <fileDependency file="CityOverhaul.esp" state="Inactive"/>
                      </dependencies>
                      <type name="NotUsable"/>
                    </pattern>
                  </patterns>
                </dependencyType>
              </typeDescriptor>
            </plugin>
            <plugin name="No descriptor"/>
          </plugins>
        </group>
      </optionalFileGroups>
    </installStep>
  </installSteps>
</config>"#;

    fn file(file: &str, state: FileState) -> Condition {
        Condition::File {
            file: file.to_owned(),
            state,
        }
    }

    #[test]
    fn reads_steps_groups_and_options() {
        let steps = parse(SAMPLE).unwrap();
        let names: Vec<_> = steps.iter().map(|step| step.name.as_str()).collect();
        assert_eq!(names, ["Version", "Patches"]);

        let brightness = &steps[0].groups[0];
        assert_eq!(brightness.name, "Brightness");
        assert_eq!(brightness.selection, Selection::ExactlyOne);
        let options: Vec<_> = brightness
            .options
            .iter()
            .map(|option| (option.name.as_str(), option.default_status))
            .collect();
        assert_eq!(
            options,
            [
                ("Default", OptionStatus::Recommended),
                ("Brighter", OptionStatus::Optional)
            ]
        );
    }

    #[test]
    fn reads_the_flags_an_option_sets() {
        let steps = parse(SAMPLE).unwrap();
        let brighter = &steps[0].groups[0].options[1];
        assert_eq!(
            brighter.sets_flags,
            [("brightness".to_owned(), "bright".to_owned())]
        );
    }

    #[test]
    fn reads_when_a_step_is_visible() {
        let steps = parse(SAMPLE).unwrap();
        assert_eq!(steps[0].visible, None);
        assert_eq!(
            steps[1].visible,
            Some(Condition::All(vec![Condition::Flag {
                name: "brightness".to_owned(),
                value: "default".to_owned(),
            }]))
        );
    }

    #[test]
    fn reads_the_rules_that_decide_an_options_status() {
        let steps = parse(SAMPLE).unwrap();
        let city = &steps[1].groups[0].options[0];
        assert_eq!(city.default_status, OptionStatus::Optional);
        assert_eq!(
            city.rules,
            [
                (
                    Condition::All(vec![
                        file("CityOverhaul.esp", FileState::Active),
                        Condition::Any(vec![
                            file("Old.esp", FileState::Missing),
                            Condition::Unknown,
                        ]),
                    ]),
                    OptionStatus::Recommended
                ),
                (
                    Condition::All(vec![file("CityOverhaul.esp", FileState::Inactive)]),
                    OptionStatus::NotUsable
                ),
            ]
        );
    }

    #[test]
    fn an_option_without_a_descriptor_is_optional() {
        let steps = parse(SAMPLE).unwrap();
        let bare = &steps[1].groups[0].options[1];
        assert_eq!(bare.default_status, OptionStatus::Optional);
        assert_eq!(bare.rules, Vec::new());
    }

    #[test]
    fn steps_groups_and_options_come_in_the_order_the_installer_shows() {
        let xml = r#"<config>
          <installSteps>
            <installStep name="beta"><optionalFileGroups order="Descending">
              <group name="A" type="SelectAny"><plugins order="Explicit">
                <plugin name="Zulu"><description>  Last letter.  </description></plugin>
                <plugin name="Alpha"/>
              </plugins></group>
              <group name="B" type="SelectAny"><plugins/></group>
            </optionalFileGroups></installStep>
            <installStep name="Alpha"/>
          </installSteps>
        </config>"#;
        let steps = parse(xml).unwrap();
        // Without an order, steps are sorted by title.
        let names: Vec<_> = steps.iter().map(|step| step.name.as_str()).collect();
        assert_eq!(names, ["Alpha", "beta"]);
        let groups: Vec<_> = steps[1]
            .groups
            .iter()
            .map(|group| group.name.as_str())
            .collect();
        assert_eq!(groups, ["B", "A"]);
        let options: Vec<_> = steps[1].groups[1]
            .options
            .iter()
            .map(|option| (option.name.as_str(), option.description.as_str()))
            .collect();
        assert_eq!(options, [("Zulu", "Last letter."), ("Alpha", "")]);
    }

    #[test]
    fn an_installer_without_steps_has_none() {
        let steps = parse("<config><moduleName>Simple</moduleName></config>").unwrap();
        assert_eq!(steps, Vec::new());
    }

    #[test]
    fn broken_xml_is_an_error() {
        assert!(matches!(
            parse("<config><installSteps>"),
            Err(FomodError::InvalidXml(_))
        ));
        assert!(matches!(parse(""), Err(FomodError::InvalidXml(_))));
    }

    #[test]
    fn decodes_the_encodings_installers_come_in() {
        let text = "<config>Füße</config>";
        let little: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let big: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();

        assert_eq!(decode(text.as_bytes()), text);
        assert_eq!(
            decode(&[&[0xEF, 0xBB, 0xBF], text.as_bytes()].concat()),
            text
        );
        assert_eq!(decode(&[&[0xFF, 0xFE], little.as_slice()].concat()), text);
        assert_eq!(decode(&[&[0xFE, 0xFF], big.as_slice()].concat()), text);
        // UTF-16 without a byte order mark.
        assert_eq!(decode(&little), text);
        assert_eq!(decode(&big), text);
        assert_eq!(decode(b""), "");
    }

    #[test]
    fn a_decoded_utf16_installer_parses() {
        let bytes: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain(SAMPLE.encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        assert_eq!(parse(&decode(&bytes)).unwrap().len(), 2);
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..400)) {
            let _ = parse(&decode(&bytes));
        }

        #[test]
        fn never_panics_on_a_damaged_installer(position in 0usize..3000, replacement in "[<>/\"=a-z ]{0,6}") {
            let mut text = SAMPLE.to_owned();
            if let Some((index, _)) = text.char_indices().nth(position) {
                text.insert_str(index, &replacement);
            }
            let _ = parse(&text);
        }
    }
}
