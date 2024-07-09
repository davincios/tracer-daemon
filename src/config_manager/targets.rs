use lazy_static::lazy_static;

use crate::config_manager::{CommandContainsStruct, Target};

lazy_static! {
    pub static ref TARGETS: Vec<Target> = [
        Target::ProcessName("python".to_string()),
        Target::CommandContains(CommandContainsStruct {
            command_content: "uk.ac.babraham.FastQC.FastQCApplication".to_string(),
            merge_with_parents: true,
            force_ancestor_to_match: false
        }),
        Target::ProcessName("STAR".to_string()),
        Target::ProcessName("bowtie2".to_string()),
        Target::ProcessName("bwa".to_string()),
        Target::ProcessName("salmon".to_string()),
        Target::ProcessName("hisat2".to_string()),
        Target::ProcessName("HOMER".to_string()),
        Target::ProcessName("samtools".to_string()),
        Target::ProcessName("bedtools".to_string()),
        Target::ProcessName("deeptools".to_string()),
        Target::ProcessName("macs3".to_string()),
        Target::ProcessName("plotCoverage".to_string()),
        Target::ProcessName("MACS33".to_string()),
        Target::ProcessName("Genrich".to_string()),
        Target::ProcessName("TopHat".to_string()),
        Target::ProcessName("JAMM".to_string()),
        Target::ProcessName("fastqc".to_string()),
        Target::ShortLivedProcessExecutable("fastqc".to_string()),
        Target::ProcessName("multiqc".to_string()),
        Target::ProcessName("fastp".to_string()),
        Target::ProcessName("PEAR".to_string()),
        Target::ProcessName("Trimmomatic".to_string()),
        Target::ProcessName("sra-toolkit".to_string()),
        Target::ProcessName("Picard".to_string()),
        Target::ProcessName("cutadapt".to_string()),
        Target::ProcessName("cellranger".to_string()),
        Target::ProcessName("STATsolo".to_string()),
        Target::ProcessName("scTE".to_string()),
        Target::ProcessName("scanpy".to_string()),
        Target::ProcessName("Seurat".to_string()),
        Target::ProcessName("LIGER".to_string()),
        Target::ProcessName("SC3".to_string()),
        Target::ProcessName("Louvain".to_string()),
        Target::ProcessName("Leiden".to_string()),
        Target::ProcessName("Garnett".to_string()),
        Target::ProcessName("Monocle".to_string()),
        Target::ProcessName("Harmony".to_string()),
        Target::ProcessName("PAGA".to_string()),
        Target::ProcessName("Palantir".to_string()),
        Target::ProcessName("velocity".to_string()),
        Target::ProcessName("CellPhoneDB".to_string()),
        Target::ProcessName("CellChat".to_string()),
        Target::ProcessName("NicheNet".to_string()),
        Target::ProcessName("FIt-SNE".to_string()),
        Target::ProcessName("umap".to_string()),
        Target::ProcessName("bbmap".to_string()),
        Target::ProcessName("cuffdiff".to_string()),
        Target::ProcessName("RNA-SeQC".to_string()),
        Target::ProcessName("RSeQC".to_string()),
        Target::ProcessName("Trimgalore".to_string()),
        Target::ProcessName("UCHIME".to_string()),
        Target::ProcessName("Erange".to_string()),
        Target::ProcessName("X-Mate".to_string()),
        Target::ProcessName("SpliceSeq".to_string()),
        Target::ProcessName("casper".to_string()),
        Target::ProcessName("DESeq".to_string()),
        Target::ProcessName("EdgeR".to_string()),
        Target::ProcessName("Kallisto".to_string()),
        Target::ProcessName("pairtools".to_string()),
        Target::ProcessName("HiCExplorer".to_string()),
        Target::ProcessName("GITAR".to_string()),
        Target::ProcessName("TADbit".to_string()),
        Target::ProcessName("Juicer".to_string()),
        Target::ProcessName("HiC-Pro".to_string()),
        Target::ProcessName("cooler".to_string()),
        Target::ProcessName("cooltools".to_string()),
        Target::ProcessName("runHiC".to_string()),
        Target::ProcessName("HTSlib".to_string()),
        Target::ProcessName("zlib".to_string()),
        Target::ProcessName("libbz2".to_string()),
        Target::ProcessName("liblzma".to_string()),
        Target::ProcessName("libcurl".to_string()),
        Target::ProcessName("libdeflate".to_string()),
        Target::ProcessName("ncurses".to_string()),
        Target::ProcessName("pthread".to_string()),
    ]
    .to_vec();
}
