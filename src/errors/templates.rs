use lazy_static::lazy_static;
use predicates::prelude::predicate;

use crate::{
    boxed_condition, condition,
    errors::{
        conditions::{
            ErrorBaseCondition, ErrorCondition, FileExistsCondition, LogContainsCondition,
            LogContainsInner, ToolRunTimeGreaterThanCondition,
        },
        ErrorSeverity,
    },
    stdout_condition,
};

use super::ErrorTemplate;

lazy_static! {
    pub static ref ERROR_TEMPLATES: Vec<ErrorTemplate> = vec![ErrorTemplate {
      id: "star_error_1".to_string(),
      display_name: "Star Error 1".to_string(),
      severity: ErrorSeverity::Critical,
      condition: ErrorCondition::And(vec![
          ErrorCondition::Not(boxed_condition!(ToolRunTimeGreaterThanCondition { tool_name: "STAR".to_string(), run_time: 10 })),
          ErrorCondition::Not(boxed_condition!(ToolRunTimeGreaterThanCondition { tool_name: "STAR".to_string(), run_time: 25 })), // TODO: It should be a genome tool run
          condition!(ToolRunTimeGreaterThanCondition { tool_name: "STAR".to_string(), run_time: 10 }),  // TODO: It should also be a genome tool run
          stdout_condition!("*EXITING because of FATAL ERROR: could not open genome file*"),
      ]),
      causes: vec!["Lack of sufficient computational resource (RAM/Cores)".to_string(),
      "Species mismatch – wrong GTF or FASTA file used for genome index creation".to_string()],
      advices: vec!["Check that the path to genome files, specified in --genomeDir is correct and the files are present, and have user read permissions".to_string(),
          "Check that enough cores/RAM has been assigned to STAR".to_string(),
      ],
    },
    ErrorTemplate {
      id: "star_error_2".to_string(),
      display_name: "Star Error 2".to_string(),
      severity: ErrorSeverity::Critical,
      condition: ErrorCondition::And(vec![
          ErrorCondition::Not(boxed_condition!(ToolRunTimeGreaterThanCondition { tool_name: "STAR".to_string(), run_time: 10 })),
          ErrorCondition::Not(boxed_condition!(ToolRunTimeGreaterThanCondition { tool_name: "STAR".to_string(), run_time: 10 })), // TODO: It should be a genome tool run
          stdout_condition!("*EXITING because of FATAL INPUT PARAMETER ERROR: when generating genome without annotations*"),
      ]),
      causes: vec!["Parameter Mismatch".to_string(),
          "SpUse of outdated genome assembly (FASTA)".to_string()],
      advices: vec!["Re-run genome generation without --sjdbOverhang option".to_string(),
          "Replace genome assembly with updated or most recent FASTA".to_string(),
          "Use an annotation GTF/GFF file to enable genome index generation genome assembly with updated or most recent FASTA file".to_string(),]
    },
    ErrorTemplate {
      id: "star_error_3".to_string(),
      display_name: "Star Error 3".to_string(),
      severity: ErrorSeverity::Critical,
      condition: ErrorCondition::And(vec![
          ErrorCondition::Not(boxed_condition!(ToolRunTimeGreaterThanCondition { tool_name: "STAR".to_string(), run_time: 10 })),
          ErrorCondition::Not(boxed_condition!(FileExistsCondition { file_path: predicate::str::is_match("*star output file.txt").unwrap() })), // TODO: Fill STAR output filename
          stdout_condition!("*EXITING because of FATAL ERROR in reads input: short read sequence*"),
      ]),
      causes: vec!["Read length shorter than expected".to_string(),
          "Wrongly formed or incorrectly formatted FASTQ file".to_string()],
      advices: vec!["Ensure all input reads meet the minimum length requirement".to_string(),
          "Use grep -A5 to survey the short reads or the format of the FASTQ file, to enable correction of the input file".to_string(),]
    },
    ErrorTemplate {
      id: "bowtie2_error_1".to_string(),
      display_name: "bowtie2 Error 1".to_string(),
      severity: ErrorSeverity::Critical,
      condition: ErrorCondition::And(vec![
          ErrorCondition::Not(boxed_condition!(ToolRunTimeGreaterThanCondition { tool_name: "bowtie2".to_string(), run_time: 10 })), // TODO: What time is 'too fast'?
          stdout_condition!("*bowtie2-align exited with value 1*"),
      ]),
      causes: vec!["Indexing error".to_string(),
          "Corrupt input file".to_string(),
          "Incorrect path specified to the generated genome index".to_string()],
      advices: vec!["Check the integrity of input files".to_string(),
          "Check that enough cores/RAM has been assigned to bowtie2".to_string(),
          "Check the specified path for the generated genome index".to_string()
      ],
    },
    ErrorTemplate {
      id: "salmon_error_1".to_string(),
      display_name: "Salmon Error 1".to_string(),
      severity: ErrorSeverity::Critical,
      condition: ErrorCondition::And(vec![
          stdout_condition!("*Quantification failed due to a kmer size mismatch*"),
      ]),
      causes: vec!["Inconsistent kmer size in index".to_string(),
          "Default kmer size of 31 not suitable for short reads".to_string(),
          "Too many short, fragmented reads in sample".to_string()],
      advices: vec!["Ensure the kmer size used during indexing matches the quantification step".to_string(),
          "Choose appropriate kmer size".to_string(),
          "Determine frequency distribution of read sizes in input dataset".to_string()
      ],
    }
    ];
}
