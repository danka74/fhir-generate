mod utils;

use crate::utils::{
    count_char_occurrences, generate_code, get_slice_after_last_occurrence, load_json_from_file,
};
use clap::{Args, Parser, Subcommand};
use easy_tree::Tree;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};
use utils::{camel_to_spaced_pascal, reduce_datatypes};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Command
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Debug)]
struct CommonArgs {
    /// Files to process
    files: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate plantUml class diagrams in a single file based on the structure definitions
    PlantUml(PlantUmlArgs),
    /// Generate a plantUml mind map in a separate file for each structure definition
    Mindmap(MindmapArgs),
    /// Generate a markdown table in a separate file for each structure definition
    Table(TableArgs),
    /// Generate a markdown table in a single file based on obligations of a structure definition
    Obligations(ObligationsArgs),
    /// Generate plantUml diagram from an instances file
    Instances(InstancesArgs),
}

#[derive(Args, Debug)]
struct InstancesArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Hide data elements
    #[arg(short, long)]
    elements_hide: bool,

    /// Output file name
    #[arg(short, long, default_value = "instances_output.plantuml")]
    output_file: PathBuf,
}

#[derive(Args, Debug)]
struct PlantUmlArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Hide data elements
    #[arg(short, long)]
    elements_hide: bool,

    /// Hide cardinality
    #[arg(short, long)]
    cardinality_hide: bool,

    /// Output tile name
    #[arg(short, long, default_value = "output.plantuml")]
    output_file: PathBuf,
}

#[derive(Args, Debug)]
struct MindmapArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// At which hierarchical level to stop using boxes in mind map
    #[arg(short, long, default_value_t = 255)]
    box_level: usize,
}

#[derive(Args, Debug)]
struct TableArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Hide code
    #[arg(short, long)]
    code_hide: bool,

    /// Prefix used for code generation
    #[arg(short, long, default_value = "A")]
    prefix_code: String,
}

#[derive(Args, Debug)]
struct ObligationsArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Prefix used for code generation
    #[arg(short, long, default_value = "A")]
    prefix_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ElementInfo {
    id: String,
    short: String,
    definition: String,
    datatype: Vec<String>,
    min: String,
    max: String,
    binding: Option<String>,
    binding_strength: Option<String>,
    obligation: Vec<(String, String)>,
    requirements: Option<String>,
}

#[derive(Debug)]
struct StructureDefInfo {
    id: String,
    elements: Vec<ElementInfo>,
}

struct StructureDefTreeInfo {
    id: String,
    element_tree: Tree<ElementInfo>,
}

#[derive(Debug)]
struct InstanceInfo {
    id: String,
    elements: Vec<ElementInfo>,
}

trait SearchableTree<T> {
    fn find_first<F>(&self, predicate: F) -> Option<usize>
        where
        F: Fn(&T) -> bool;
}

impl SearchableTree<ElementInfo> for Tree<ElementInfo> {
    fn find_first<F>(&self, predicate: F) -> Option<usize>
        where
        F: Fn(&ElementInfo) -> bool
    {
        for node in self.iter() {
            if predicate(node.1) {
                return Some(node.0);
            }
        }
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::PlantUml(args) => {
            // first load all structure definitions into in-memory structs
            let docs = load_structure_definition_files(&args.common.files)?;
            let output = File::create(args.output_file)?;
            let mut writer = BufWriter::new(output); // Create a buffered writer

            writeln!(
                writer,
                "@startuml\nskinparam linetype polyline\nhide circle\nhide stereotype\nhide methods\n"
            )?;

            for doc in docs.iter() {
                println!("processing: {}", doc.id);
                writeln!(writer, "class **{}** {{", doc.id)?;
                let mut relations = String::new();

                // let mut _element_number = 0;

                // TODO: Assumes elements appear in the right order. Could sort on ElementDefinition.id but that would change the stated order.
                let mut sorted_elements = doc.elements.clone();
                sorted_elements.sort_by(|a, b| a.id.cmp(&b.id));
                for element in sorted_elements.iter() {
                    if let Some(element_part) = get_slice_after_last_occurrence(&element.id, '.') {
                        if element.max == "0" {
                            // do not show elements with max cardinality 0
                            continue;
                        }
                        let hier_level = count_char_occurrences(&element.id, '.') * 2;
                        // if the datatype is one of the classes drawn, add a relation instead of a class element
                        // TODO: element is removed from element list if there is one datatype that is among the structure definitions
                        let mut show_element = true;
                        if element_part.ends_with("[x]") {
                            let clean_part = element_part.replace("[x]", "");
                            let choice = format!("{}{}", doc.id, clean_part);
                            let mut local_relations = String::new();
                            for datatype in element.datatype.iter() {
                                // TODO: or use a hashmap for faster lookup
                                // TODO: look also for Reference(X or T)
                                if docs.iter().any(|d| datatype == &d.id) {
                                    local_relations += &format!(
                                        "{} .. \"**{}**\" : {} >\n",
                                        choice, datatype, clean_part
                                    );
                                    // will hade element if there is just one datatype that is another class in the diagram,
                                    show_element = false; // do not show element if it is a choice
                                }
                            }
                            if !show_element {
                                relations += &format!("<> {}\n", choice);
                                relations += &format!(
                                    "\"**{}**\" -- \"{}..{}\" {} : {} >\n",
                                    doc.id, element.min, element.max, choice, clean_part
                                );
                                relations += &local_relations;
                            }
                        } else {
                            for datatype in element.datatype.iter() {
                                // TODO: or use a hashmap for faster lookup
                                // TODO: look also for Reference(X or T)
                                if docs.iter().any(|d| datatype == &d.id) {
                                    relations += &format!(
                                        "\"**{}**\" -- \"{}..{}\" \"**{}**\" : {} >\n",
                                        doc.id, element.min, element.max, datatype, element_part
                                    );
                                    show_element = false; // do not show element if datatype is another class in the diagram
                                }
                            }
                        }

                        if show_element && !args.elements_hide {
                            write!(
                                writer,
                                "{:>hier_level$}|_ {} : {}",
                                "",
                                element_part,
                                reduce_datatypes(&element.datatype)
                            )?;
                            if !args.cardinality_hide {
                                write!(writer, " [{}..{}]", element.min, element.max)?;
                            }
                            writeln!(writer)?;
                        }
                    }
                }

                writeln!(writer, "}}")?;

                write!(writer, "{}", relations)?;
            }

            writeln!(writer, "@enduml")?;
        }
        Commands::Mindmap(args) => {
            // first load all structure definitions into in-memory structs
            let docs = load_structure_definition_files(&args.common.files)?;
            for doc in docs.iter() {
                println!("processing: {}", doc.id);
                let output = File::create(format!("{}.plantuml", doc.id))?;
                let mut writer = BufWriter::new(output); // Create a buffered writer

                writeln!(writer, "@startmindmap")?;

                writeln!(writer, "* {}", doc.id)?;

                for element in doc.elements.iter() {
                    if element.max == "0" {
                        // do not show elements with max cardinality 0
                        continue;
                    }
                    if let Some(element_part) = get_slice_after_last_occurrence(&element.id, '.') {
                        let hier_level: usize = count_char_occurrences(&element.id, '.') + 1;
                        writeln!(
                            writer,
                            "{}{} {}",
                            "*".repeat(hier_level),
                            if hier_level > args.box_level { "_" } else { "" },
                            camel_to_spaced_pascal(&element_part.replace("[x]", ""))
                        )?;
                    }
                }

                writeln!(writer, "@endmindmap")?;
            }
        }
        Commands::Table(args) => {
            // first load all structure definitions into in-memory structs
            let docs = load_structure_definition_files(&args.common.files)?;
            let alpha_index_code = args.prefix_code == "A";
            for (doc_num, doc) in docs.iter().enumerate() {
                let prefix = if alpha_index_code {
                    generate_code(doc_num)
                } else {
                    args.prefix_code.clone()
                };
                println!("processing: {}", doc.id);
                let output = File::create(format!("{}.md", doc.id))?;
                let mut writer = BufWriter::new(output); // Create a buffered writer

                writeln!(writer, "# {}", doc.id)?;

                writeln!(
                    writer,
                    "| Code | Path | Element | Short | Definition | Datatype | Cardinality | Preferred Code System | Binding Strength | Requirements |"
                )?;
                writeln!(
                    writer,
                    "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- "
                )?;

                let mut levels = Vec::<usize>::new();
                levels.push(0);
                let mut current_level: usize = 0;
                let mut sorted_elements = doc.elements.clone();
                // sorted_elements.sort_by(|a, b| a.id.cmp(&b.id));
                for element in sorted_elements.iter() {
                    let hier_level: usize = count_char_occurrences(&element.id, '.');
                    let element_part: String = if hier_level > 0 {
                        get_slice_after_last_occurrence(&element.id, '.')
                            .ok_or("Wrong element part")?
                    } else {
                        element.id.clone()
                    };
                    // if (hier_level as isize - current_level as isize).abs() > 1 {
                    //     return Err(format!("Hierarchical level difference is too large: {}", element.id).into());
                    // }
                    match hier_level.cmp(&current_level) {
                        std::cmp::Ordering::Greater => {
                            levels.push(1);
                            current_level += 1;
                        }
                        std::cmp::Ordering::Less => {
                            levels.pop();
                            current_level -= 1;
                            levels[current_level] += 1;
                        }
                        std::cmp::Ordering::Equal => {
                            levels[current_level] += 1;
                        }
                    }
                    let mut code = prefix.clone();
                    for level in &levels[1..=current_level] {
                        code.push('.');
                        code.push_str(&level.to_string());
                    }

                    let clean_part = element_part.replace("[x]", "");
                    write!(
                        writer,
                        "| {} | {}{} | {} | {} | {} | {} | {}..{} |",
                        code,
                        ".".repeat(current_level),
                        clean_part,
                        camel_to_spaced_pascal(&clean_part),
                        element.short,
                        element.definition.replace("\n", "<br/>"),
                        reduce_datatypes(&element.datatype),
                        element.min,
                        element.max
                    )?;
                    if let Some(binding) = &element.binding {
                        write!(writer, " {} |", binding)?;
                    } else {
                        write!(writer, " |")?;
                    }
                    if let Some(binding_strength) = &element.binding_strength {
                        write!(writer, " {} |", binding_strength)?;
                    } else {
                        write!(writer, " |")?;
                    }
                    if let Some(requirements) = &element.requirements {
                        write!(writer, " {} |", requirements.replace("\n", "<br/>"))?;
                    } else {
                        write!(writer, " |")?;
                    }
                    writeln!(writer)?;
                }
            }
        }
        Commands::Obligations(args) => {
            // first load all structure definitions into in-memory structs
            let docs: Vec<StructureDefInfo> = load_structure_definition_files(&args.common.files)?;
            for doc in docs.iter() {
                println!("processing: {}", doc.id);
                let output = File::create(format!("{}.md", doc.id))?;
                let mut writer = BufWriter::new(output); // Create a buffered writer

                writeln!(writer, "# {}", doc.id)?;

                let mut table = Vec::<(String, HashMap<String, Vec<String>>)>::new();
                let mut unique_actors = HashSet::<String>::new();

                for element in doc.elements.iter() {
                    if element.max == "0" {
                        // do not show elements with max cardinality 0
                        continue;
                    }
                    let hier_level: usize = count_char_occurrences(&element.id, '.');
                    let element_part: String = if hier_level > 0 {
                        get_slice_after_last_occurrence(&element.id, '.')
                            .ok_or("Wrong element part")?
                    } else {
                        element.id.clone()
                    };

                    if element.obligation.is_empty() {
                        continue;
                    }

                    table.push((element_part.clone(), HashMap::<String, Vec<String>>::new()));

                    for obligation in &element.obligation {
                        let hash = &mut table.last_mut().ok_or("Error in obligations list")?.1;
                        let actor = obligation.0.clone();
                        unique_actors.insert(actor.clone());
                        let code = obligation.1.clone();
                        let codes = hash.entry(actor).or_insert(Vec::new());
                        codes.push(code);
                    }
                }

                let no_of_actors = unique_actors.len();

                write!(writer, "| Element ")?;
                for actor in unique_actors.iter() {
                    let actor_name =
                        get_slice_after_last_occurrence(actor, '/').ok_or("Wrong actor URL")?;
                    write!(writer, "| {} ", actor_name)?;
                }
                writeln!(writer, "|")?;
                write!(writer, "| --- ")?;
                for _ in 0..no_of_actors {
                    write!(writer, "| --- ")?;
                }
                writeln!(writer, "|")?;

                for (element, hash) in table.iter() {
                    write!(writer, "| {} ", camel_to_spaced_pascal(element))?;
                    for actor in unique_actors.iter() {
                        if let Some(codes) = hash.get(actor) {
                            write!(writer, "| {} ", codes.join(", "))?;
                        } else {
                            write!(writer, "| ")?;
                        }
                    }
                    writeln!(writer, "|")?;
                }
            }
        }
        Commands::Instances(args) => {
            let docs: Vec<InstanceInfo> = load_instance_files(&args.common.files)?;
            let output = File::create(args.output_file)?;
            let mut writer = BufWriter::new(output); // Create a buffered writer
            writeln!(
                writer,
                "@startuml\nskinparam linetype polyline\nhide circle\nhide stereotype\nhide methods\n"
            )?;

            for doc in docs.iter() {
                println!("processing: {}", doc.id);
            }
            writeln!(writer, "@enduml")?;
        }
    }

    Ok(())
}

fn load_instance_files(files: &[String]) -> Result<Vec<InstanceInfo>, Box<dyn std::error::Error>> {
    let mut docs = Vec::<InstanceInfo>::new();

    for file in files.iter() {
        match load_single_instance_file(file) {
            Ok(doc_info) => {
                docs.push(doc_info);
            }
            Err(e) => {
                println!("Error reading file '{}': {}", file, e);
            }
        }
    }
    Ok(docs)
}

fn load_single_instance_file(_file: &str) -> Result<InstanceInfo, Box<dyn std::error::Error>> {
    todo!()
}

fn load_structure_definition_files(
    files: &[String],
) -> Result<Vec<StructureDefInfo>, Box<dyn std::error::Error>> {
    let mut docs = Vec::<StructureDefInfo>::new();
    for file in files.iter() {
        match load_single_structure_definition_file(file) {
            Ok(doc_info) => {
                docs.push(doc_info);
            }
            Err(e) => {
                println!("Error reading file '{}': {}", file, e);
            }
        }
    }
    Ok(docs)
}

fn load_single_structure_definition_file(
    file: &String,
) -> Result<StructureDefInfo, Box<dyn std::error::Error>> {
    let doc = load_json_from_file(file)?;
    let id = doc["id"].as_str().ok_or("Missing id")?;
    let snapshot = doc["snapshot"]["element"]
        .as_array()
        .ok_or("Missing snapshot")?;
    let mut elements = Vec::<ElementInfo>::new();
    for element in snapshot.iter() {
        let element_id = element["id"].as_str().ok_or("Missing element id")?;
        let short = element["short"]
            .as_str()
            .ok_or("Missing short description")?
            .to_string();
        let definition = element["definition"]
            .as_str()
            .ok_or("Missing definition")?
            .to_string();
        let requirements = element["requirements"].as_str().map(|s| s.to_string());

        let mut datatype = Vec::<String>::new();
        if let Some(type_array) = element["type"].as_array() {
            for dt in type_array {
                if let Some(code) = dt["code"].as_str() {
                    let code = code.to_string();
                    if code.starts_with("http") {
                        if let Some(end) = get_slice_after_last_occurrence(&code, '/') {
                            datatype.push(end);
                        };
                    } else if code == "Reference" {
                        // TODO: does not distinguish between Reference and direct datatype
                        if let Some(profiles) = dt["targetProfile"].as_array() {
                            for profile_value in profiles {
                                if let Some(profile) = profile_value.as_str() {
                                    let profile = profile.to_string();
                                    if let Some(end) =
                                        get_slice_after_last_occurrence(&profile, '/')
                                    {
                                        datatype.push(end);
                                    };
                                }
                            }
                        }
                    } else {
                        datatype.push(code);
                    }
                }
            }
        }

        let mut obligation = Vec::<(String, String)>::new();
        if let Some(ext_array) = element["extension"].as_array() {
            for ext in ext_array {
                if ext["url"].as_str() == Some("http://hl7.org/fhir/StructureDefinition/obligation")
                {
                    let mut code = String::new();
                    let mut actor = String::new();
                    if let Some(ext2_array) = ext["extension"].as_array() {
                        for ext2 in ext2_array {
                            if ext2["url"].as_str() == Some("code") {
                                if let Some(value) = ext2["valueCode"].as_str() {
                                    code = value.to_string();
                                }
                            } else if ext2["url"].as_str() == Some("actor") {
                                if let Some(value) = ext2["valueCanonical"].as_str() {
                                    actor = value.to_string();
                                }
                            }
                        }
                    }
                    if !code.is_empty() && !actor.is_empty() {
                        obligation.push((actor, code));
                    }
                }
            }
        }

        let min = if element["min"].is_string() {
            element["min"]
                .as_str()
                .ok_or(format!("Missing min cardinality: {:?}", element["min"]))?
                .to_string()
        } else {
            element["min"].to_string()
        };

        let max = element["max"].as_str().ok_or("Missing max cardinality")?;
        let binding = element["binding"]["description"]
            .as_str()
            .map(|s| s.to_string());
        let binding_strength = element["binding"]["strength"]
            .as_str()
            .map(|s| s.to_string());

        elements.push(ElementInfo {
            id: element_id.to_string(),
            short,
            definition,
            datatype,
            min,
            max: max.to_string(),
            binding,
            binding_strength,
            obligation,
            requirements,
        });

        
    }
    Ok(StructureDefInfo {
        id: id.to_string(),
        elements,
    })
}

fn load_single_structure_definition_file_into_tree(
    file: &String,
) -> Result<StructureDefTreeInfo, Box<dyn std::error::Error>> {
    let doc = load_json_from_file(file)?;
    let id = doc["id"].as_str().ok_or("Missing id")?;
    let snapshot = doc["snapshot"]["element"]
        .as_array()
        .ok_or("Missing snapshot")?;
    let mut element_tree: Tree<ElementInfo> = Tree::new();
    // let mut elements = Vec::<ElementInfo>::new();
    for element in snapshot.iter() {
        let element_id = element["id"].as_str().ok_or("Missing element id")?;
        let parent_id = if let Some(last_index) = element_id.rfind('.') {
            Some(&element_id[..last_index])
        } else {
            None
        };
        let parent_node = if let Some(pid) = parent_id {
            element_tree.find_first(|e| e.id == pid)
        } else {
            None
        };
        let short = element["short"]
            .as_str()
            .ok_or("Missing short description")?
            .to_string();
        let definition = element["definition"]
            .as_str()
            .ok_or("Missing definition")?
            .to_string();
        let requirements = element["requirements"].as_str().map(|s| s.to_string());

        let mut datatype = Vec::<String>::new();
        if let Some(type_array) = element["type"].as_array() {
            for dt in type_array {
                if let Some(code) = dt["code"].as_str() {
                    let code = code.to_string();
                    if code.starts_with("http") {
                        if let Some(end) = get_slice_after_last_occurrence(&code, '/') {
                            datatype.push(end);
                        };
                    } else if code == "Reference" {
                        // TODO: does not distinguish between Reference and direct datatype
                        if let Some(profiles) = dt["targetProfile"].as_array() {
                            for profile_value in profiles {
                                if let Some(profile) = profile_value.as_str() {
                                    let profile = profile.to_string();
                                    if let Some(end) =
                                        get_slice_after_last_occurrence(&profile, '/')
                                    {
                                        datatype.push(end);
                                    };
                                }
                            }
                        }
                    } else {
                        datatype.push(code);
                    }
                }
            }
        }

        let mut obligation = Vec::<(String, String)>::new();
        if let Some(ext_array) = element["extension"].as_array() {
            for ext in ext_array {
                if ext["url"].as_str() == Some("http://hl7.org/fhir/StructureDefinition/obligation")
                {
                    let mut code = String::new();
                    let mut actor = String::new();
                    if let Some(ext2_array) = ext["extension"].as_array() {
                        for ext2 in ext2_array {
                            if ext2["url"].as_str() == Some("code") {
                                if let Some(value) = ext2["valueCode"].as_str() {
                                    code = value.to_string();
                                }
                            } else if ext2["url"].as_str() == Some("actor") {
                                if let Some(value) = ext2["valueCanonical"].as_str() {
                                    actor = value.to_string();
                                }
                            }
                        }
                    }
                    if !code.is_empty() && !actor.is_empty() {
                        obligation.push((actor, code));
                    }
                }
            }
        }

        let min = if element["min"].is_string() {
            element["min"]
                .as_str()
                .ok_or(format!("Missing min cardinality: {:?}", element["min"]))?
                .to_string()
        } else {
            element["min"].to_string()
        };

        let max = element["max"].as_str().ok_or("Missing max cardinality")?;
        let binding = element["binding"]["description"]
            .as_str()
            .map(|s| s.to_string());
        let binding_strength = element["binding"]["strength"]
            .as_str()
            .map(|s| s.to_string());

        if let Some(parent) = parent_node {
            element_tree.add_child(
                parent,
                ElementInfo {
                    id: element_id.to_string(),
                    short: short.clone(),
                    definition: definition.clone(),
                    datatype: datatype.clone(),
                    min: min.clone(),
                    max: max.to_string(),
                    binding: binding.clone(),
                    binding_strength: binding_strength.clone(),
                    obligation: obligation.clone(),
                    requirements: requirements.clone(),
                },
            );
        } else {
            element_tree.add_node(ElementInfo {
                id: element_id.to_string(),
                short: short.clone(),
                definition: definition.clone(),
                datatype: datatype.clone(),
                min: min.clone(),
                max: max.to_string(),
                binding: binding.clone(),
                binding_strength: binding_strength.clone(),
                obligation: obligation.clone(),
                requirements: requirements.clone(),
            });
        }
        
    }
    Ok(StructureDefTreeInfo {
        id: id.to_string(),
        element_tree,
    })
}
