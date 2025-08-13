use feynman_api::router::ApiDoc;
use utoipa::OpenApi;

/// Generates the OpenAPI specification and writes it to a file.
fn generate_spec(
    api_doc: utoipa::openapi::OpenApi,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec_json = api_doc.to_pretty_json()?;
    std::fs::write(path, spec_json)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_spec(ApiDoc::openapi(), "openapi.json")?;
    Ok(())
}
