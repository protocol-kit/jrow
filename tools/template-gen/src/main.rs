//! JROW Template Generator
//!
//! This tool generates deployment configurations and documentation for JROW
//! applications from templates. It produces:
//!
//! - **Docker**: Dockerfile and docker-compose.yml
//! - **Kubernetes**: Deployment and ConfigMap manifests
//! - **AsyncAPI**: API documentation in AsyncAPI format
//! - **Scripts**: Deployment automation scripts
//!
//! # Usage
//!
//! ```bash
//! # Generate default configuration file
//! cargo run -p template-gen
//!
//! # Edit jrow-template.toml with your settings
//!
//! # Generate deployment files
//! cargo run -p template-gen
//!
//! # Or with custom paths
//! cargo run -p template-gen -- --config myconfig.toml --output ./deploy
//! ```
//!
//! # Configuration
//!
//! The tool reads settings from a TOML file (default: `jrow-template.toml`)
//! containing:
//! - Project metadata (name, version, description)
//! - Server configuration (ports, batch mode)
//! - Docker settings (image name, registry)
//! - Kubernetes settings (replicas, resources)
//! - AsyncAPI documentation (methods, topics)
//!
//! # Output Structure
//!
//! Generated files are organized as:
//! ```text
//! deploy/
//! ├── docker/
//! │   ├── Dockerfile
//! │   └── docker-compose.yml
//! ├── k8s/
//! │   ├── deployment.yaml
//! │   └── configmap.yaml
//! ├── scripts/
//! │   └── deploy.sh
//! ├── asyncapi.yaml
//! └── README.md
//! ```
//!
//! # Templates
//!
//! Templates are embedded in the binary using `include_str!()` and rendered
//! using the Tera template engine with the configuration as context.

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tera::Tera;

#[derive(Parser, Debug)]
#[command(author, version, about = "Generate deployment configs from JROW templates", long_about = None)]
struct Args {
    /// Path to the template configuration file
    #[arg(short, long, default_value = "jrow-template.toml")]
    config: PathBuf,

    /// Output directory for generated files
    #[arg(short, long, default_value = "deploy")]
    output: PathBuf,

    /// Path to JROW templates directory
    #[arg(short, long, default_value = "templates/deploy")]
    templates: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct TemplateConfig {
    project: ProjectConfig,
    server: ServerConfig,
    docker: DockerConfig,
    kubernetes: KubernetesConfig,
    asyncapi: AsyncApiConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectConfig {
    name: String,
    description: String,
    version: String,
    rust_version: String,
    license: String,
    license_url: Option<String>,
    contact_name: Option<String>,
    contact_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerConfig {
    bind_address: String,
    port: u16,
    batch_mode: String,
    max_connections: u32,
    connection_timeout: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct DockerConfig {
    image_name: String,
    registry: Option<String>,
    expose_ports: Vec<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
struct KubernetesConfig {
    namespace: String,
    replicas: u32,
    service_type: String,
    resources: ResourcesConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourcesConfig {
    requests_memory: String,
    requests_cpu: String,
    limits_memory: String,
    limits_cpu: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AsyncApiConfig {
    production_host: String,
    production_port: u16,
    production_protocol: String,
    development_host: String,
    development_port: u16,
    development_protocol: String,
    security_enabled: bool,
    methods: Vec<RpcMethod>,
    topics: Vec<PubSubTopic>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcMethod {
    name: String,
    example_params: Option<String>,
    example_result: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PubSubTopic {
    name: String,
    example_params: Option<String>,
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig {
                name: "my-jrow-app".to_string(),
                description: "My JROW-based application".to_string(),
                version: "0.1.0".to_string(),
                rust_version: "1.75".to_string(),
                license: "MIT".to_string(),
                license_url: Some("https://opensource.org/licenses/MIT".to_string()),
                contact_name: Some("API Support".to_string()),
                contact_url: None,
            },
            server: ServerConfig {
                bind_address: "0.0.0.0".to_string(),
                port: 8080,
                batch_mode: "Parallel".to_string(),
                max_connections: 1000,
                connection_timeout: 300,
            },
            docker: DockerConfig {
                image_name: "my-jrow-app".to_string(),
                registry: None,
                expose_ports: vec![8080],
            },
            kubernetes: KubernetesConfig {
                namespace: "default".to_string(),
                replicas: 3,
                service_type: "LoadBalancer".to_string(),
                resources: ResourcesConfig {
                    requests_memory: "64Mi".to_string(),
                    requests_cpu: "100m".to_string(),
                    limits_memory: "128Mi".to_string(),
                    limits_cpu: "500m".to_string(),
                },
            },
            asyncapi: AsyncApiConfig {
                production_host: "api.example.com".to_string(),
                production_port: 443,
                production_protocol: "wss".to_string(),
                development_host: "localhost".to_string(),
                development_port: 8080,
                development_protocol: "ws".to_string(),
                security_enabled: true,
                methods: vec![
                    RpcMethod {
                        name: "add".to_string(),
                        example_params: Some(r#"{"a": 5, "b": 3}"#.to_string()),
                        example_result: Some("8".to_string()),
                    },
                    RpcMethod {
                        name: "echo".to_string(),
                        example_params: Some(r#"{"message": "hello"}"#.to_string()),
                        example_result: Some(r#"{"echoed": "hello"}"#.to_string()),
                    },
                ],
                topics: vec![
                    PubSubTopic {
                        name: "example.topic".to_string(),
                        example_params: Some(r#"{"data": "value"}"#.to_string()),
                    },
                ],
            },
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load or create config
    let config: TemplateConfig = if args.config.exists() {
        let config_content = fs::read_to_string(&args.config)
            .context("Failed to read config file")?;
        toml::from_str::<TemplateConfig>(&config_content)
            .context("Failed to parse config file")?
    } else {
        println!("Config file not found, creating default: {}", args.config.display());
        let default_config = TemplateConfig::default();
        let config_toml = toml::to_string_pretty(&default_config)?;
        fs::write(&args.config, config_toml)?;
        println!("Edit {} and run again to generate deployment files", args.config.display());
        return Ok(());
    };

    // Load templates - create Tera instance and add templates manually
    let mut tera = Tera::default();
    
    // Add embedded templates
    tera.add_raw_templates(vec![
        ("Dockerfile", include_str!("../../../templates/deploy/docker/Dockerfile.tera")),
        ("docker-compose.yml", include_str!("../../../templates/deploy/docker/docker-compose.yml.tera")),
        ("deployment.yaml", include_str!("../../../templates/deploy/k8s/deployment.yaml.tera")),
        ("configmap.yaml", include_str!("../../../templates/deploy/k8s/configmap.yaml.tera")),
        ("deploy.sh", include_str!("../../../templates/deploy/scripts/deploy.sh.tera")),
        ("README.md", include_str!("../../../templates/deploy/README.md.tera")),
        ("asyncapi.yaml", include_str!("../../../templates/asyncapi.yaml.tera")),
    ])
        .context("Failed to load templates")?;

    // Create context from config
    let mut context = tera::Context::new();
    context.insert("project", &config.project);
    context.insert("server", &config.server);
    context.insert("docker", &config.docker);
    context.insert("kubernetes", &config.kubernetes);
    context.insert("asyncapi", &config.asyncapi);
    
    // Additional computed values
    let full_image = if let Some(registry) = &config.docker.registry {
        format!("{}/{}", registry, config.docker.image_name)
    } else {
        config.docker.image_name.clone()
    };
    context.insert("full_image", &full_image);
    context.insert("bind_full", &format!("{}:{}", config.server.bind_address, config.server.port));

    // Create output directories
    fs::create_dir_all(&args.output)?;
    fs::create_dir_all(args.output.join("docker"))?;
    fs::create_dir_all(args.output.join("k8s"))?;
    fs::create_dir_all(args.output.join("scripts"))?;

    // Render templates
    println!("Generating deployment files for: {}", config.project.name);
    
    render_template(&tera, &context, "Dockerfile", &args.output.join("docker/Dockerfile"))?;
    render_template(&tera, &context, "docker-compose.yml", &args.output.join("docker/docker-compose.yml"))?;
    render_template(&tera, &context, "deployment.yaml", &args.output.join("k8s/deployment.yaml"))?;
    render_template(&tera, &context, "configmap.yaml", &args.output.join("k8s/configmap.yaml"))?;
    render_template(&tera, &context, "deploy.sh", &args.output.join("scripts/deploy.sh"))?;
    render_template(&tera, &context, "README.md", &args.output.join("README.md"))?;
    render_template(&tera, &context, "asyncapi.yaml", &args.output.join("asyncapi.yaml"))?;
    
    // Make deploy.sh executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script_path = args.output.join("scripts/deploy.sh");
        let mut perms = std::fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms)?;
        println!("  ✓ Set executable permissions on deploy.sh");
    }

    // Copy deploy script (no template needed) - if it exists
    let script_src = args.templates.join("scripts/deploy.sh");
    if script_src.exists() {
        fs::copy(&script_src, args.output.join("scripts/deploy.sh"))
            .context("Failed to copy deploy script")?;
        println!("  ✓ Copied: scripts/deploy.sh");
    }

    println!("\n✅ Deployment files generated in: {}", args.output.display());
    println!("\nNext steps:");
    println!("  1. Review generated files in {}/", args.output.display());
    println!("  2. Validate AsyncAPI spec: asyncapi validate {}/asyncapi.yaml", args.output.display());
    println!("  3. Build your Docker image: docker build -f {}/docker/Dockerfile .", args.output.display());
    println!("  4. Deploy: kubectl apply -f {}/k8s/", args.output.display());

    Ok(())
}

fn render_template(tera: &Tera, context: &tera::Context, template_name: &str, output_path: &Path) -> Result<()> {
    let rendered = tera.render(template_name, context)
        .with_context(|| format!("Failed to render template: {}", template_name))?;
    
    fs::write(output_path, rendered)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
    
    println!("  ✓ Generated: {}", output_path.display());
    Ok(())
}

