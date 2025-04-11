use std::{collections::{HashMap, HashSet}, env, fs::File, path::Path};

use iri_s::IriS;
use oxrdf::{IriParseError, NamedNode, Subject, Term};
use prefixmap::{PrefixMap, PrefixMapError};
use serde::Serialize;
use srdf::{SRDFBasic, SRDFGraph, SRDFGraphError, SRDF};

#[derive(Serialize)]
struct Matches {
    matches: Vec<MatchItem>
}

#[derive(Serialize, Debug)]
struct MatchItem {
    trigger: String,
    replace: String,
    label: String,
}

#[derive(Debug)]
struct Item {
    qualified_name: String,
    english_label: Option<String>,
}

enum SubjectType {
    Class,
    ObjectProperty
}

fn find_subjects(graph: &SRDFGraph, pred: &<SRDFGraph as SRDFBasic>::IRI, object: &<SRDFGraph as SRDFBasic>::Term) -> Result<Vec<Item>, AppError> {
    let pm = PrefixMap::from_hashmap(&HashMap::from([
        ("iof-core", "https://spec.industrialontologies.org/ontology/core/Core/"),
        ("owl", "http://www.w3.org/2002/07/owl#"),
        ("bfo", "http://purl.obolibrary.org/obo/"),
    ]))?;
    let rdfs_label = NamedNode::new("http://www.w3.org/2000/01/rdf-schema#label")?;

    let mut items: Vec<Item> = Vec::new();
    for subject in graph.subjects_with_predicate_object(pred, object)? {
        let labels = graph.objects_for_subject_predicate(&subject, &rdfs_label)?;
        let english_label = get_english_label(&labels);

        let subj_iri = match subject {
            oxrdf::Subject::NamedNode(named_node) => IriS::from_named_node(&named_node),
            _ => continue
        };
        let qualified_name = pm.qualify(&subj_iri);

        items.push(Item {
            qualified_name: qualified_name.clone(),
            english_label: match english_label {
                Some(l) => Some(l.to_string()),
                None => None,
            },
        });
    }
    Ok(items)
}

fn get_english_label(labels: &HashSet<Term>) -> Option<&str> {
    for label in labels {
        let literal_content  = match label {
            oxrdf::Term::Literal(l) => l,
            _ => return None
        };

        if let Some(lang) = literal_content.language() {
            if lang == "en" {
                return Some(literal_content.value());
            }
        }
    }
    None
}

// Return a shortname for the name.
// If there are multiple words, the first letter of each word is used.
fn get_shortname(name: String) -> String {
    todo!()
}

fn get_bfo_short_number(name: &str) -> Result<i32, AppError> {
    let tokens: Vec<&str> = name.split("_").collect();
    if tokens.len() != 2 {
        return Err(AppError::BfoNameParseError);
    }
    let number = tokens.get(1).unwrap().parse::<i32>().map_err(|_| AppError::BfoNameParseError)?;
    Ok(number)
}

fn get_bfo_short_name(label: &str) -> String {
    label.split(" ").flat_map(|t| t.chars().nth(0)).collect()
}


fn build_bfo_matches(subjects: Vec<Item>, subject_type: SubjectType) -> Result<Vec<MatchItem>, AppError> {
    let mut match_items: Vec<MatchItem> = Vec::new();
    for subject in subjects {
        println!("{:#?}", subject);

        let label = match subject_type {
            SubjectType::Class => format!("bfo:{} (Class; {})", subject.english_label.clone().unwrap().replace(" ", "-"), subject.qualified_name.clone()),
            SubjectType::ObjectProperty => format!("bfo:{} (Object Property; {})", subject.english_label.clone().unwrap().replace(" ", "-"), subject.qualified_name.clone()),
        };

        // Number-based trigger.
        // e.g. :bfo-30 for object (http://purl.obolibrary.org/obo/BFO_0000030).
        let short_number = get_bfo_short_number(&subject.qualified_name)?;
        let match_item = MatchItem {
            trigger: format!(":bfo-{}", short_number),
            replace: subject.qualified_name.clone(),
            label: label.clone(),
        };
        println!("{:#?}", match_item);
        match_items.push(match_item);
        let match_item = MatchItem {
            trigger: format!(":bfo-{}", short_number),
            replace: format!("bfo:{}", subject.english_label.clone().unwrap().replace(" ", "-")),
            label: label.clone(),
        };
        println!("{:#?}", match_item);
        match_items.push(match_item);

        // Shortname-based trigger.
        // e.g. :bfo-obj for object (http://purl.obolibrary.org/obo/BFO_0000030).
        let short_name = get_bfo_short_name(&subject.english_label.clone().unwrap());
        let match_item = MatchItem {
            trigger: format!(":bfo-{}", short_name),
            replace: subject.qualified_name.clone(),
            label: label.clone(),
        };
        println!("{:#?}", match_item);
        match_items.push(match_item);
        let match_item = MatchItem {
            trigger: format!(":bfo-{}", short_name),
            replace: format!("bfo:{}", subject.english_label.clone().unwrap().replace(" ", "-")),
            label: label,
        };
        println!("{:#?}", match_item);
        match_items.push(match_item);
    }
    Ok(match_items)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("usage: {} <FILE> <PREFIX>", args.get(0).unwrap());
        std::process::exit(1);
    }

    let filename = match args.get(1) {
        Some(f) => f,
        None => std::process::exit(1),
    };
    let path = Path::new(filename);

    let prefix = match args.get(2) {
        Some(p) => p,
        None => std::process::exit(1),
    };

    let graph = SRDFGraph::from_path(path, &srdf::RDFFormat::RDFXML, None, &srdf::ReaderMode::Lax).unwrap();
    
    println!("Graph's len: {}", graph.len());

    let owl_import = NamedNode::new("http://www.w3.org/2002/07/owl#imports").unwrap();
    let rdf_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();
    let owl_class = NamedNode::new("http://www.w3.org/2002/07/owl#Class").unwrap();
    let owl_object_property = NamedNode::new("http://www.w3.org/2002/07/owl#ObjectProperty").unwrap();

    println!("\n\nImports:");
    for triple in graph.triples_with_predicate(&owl_import).unwrap() {
        println!("{}", triple);
    }

    let use_label_when_possible: bool = true;

    let mut items: Vec<MatchItem> = Vec::new();
    let result = find_subjects(&graph, &rdf_type, &Term::from(owl_class));
    match result {
        Ok( subjects) => items.append(&mut build_bfo_matches(subjects, SubjectType::Class).unwrap()),
        Err(_) => panic!("failed to find subjects for Class")
    }

    let result = find_subjects(&graph, &rdf_type, &Term::from(owl_object_property));
    match result {
        Ok( subjects) => items.append(&mut build_bfo_matches(subjects, SubjectType::ObjectProperty).unwrap()),
        Err(_) => panic!("failed to find subjects for Object Property")
    }

    let out_filepath = Path::new("packages.yml");
    let out_file = match File::create(&out_filepath) {
        Err(why) => panic!("couldn't open {}: {}", out_filepath.display(), why),
        Ok(file) => file,
    };
    
    match serde_yml::to_writer(out_file, &Matches { matches: items }) {
        Err(why) => panic!("couldn't write YAML data: {}", why),
        Ok(_) => println!("Write completed."),
    }
}

#[derive(Debug)]
enum AppError {
    AppError,
    BfoNameParseError,
}

impl From<PrefixMapError> for AppError {
    fn from(_: PrefixMapError) -> Self {
        AppError::AppError
    }
}

impl From<IriParseError> for AppError {
    fn from(_: IriParseError) -> Self {
        AppError::AppError
    }
}

impl From<SRDFGraphError> for AppError {
    fn from(_: SRDFGraphError) -> Self {
        AppError::AppError
    }
}