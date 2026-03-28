---
name: salvo-file-handling
description: Handle file uploads (single/multiple), downloads, and multipart forms. Use for file management, image uploads, and content delivery.
version: 0.89.3
tags: [data, file-upload, multipart, download]
---

# Salvo File Handling

This skill helps handle file uploads and downloads in Salvo applications.

## Setup

```toml
[dependencies]
salvo = "0.89.3"
tokio = { version = "1", features = ["fs"] }
```

## Single File Upload

```rust
use std::path::Path;
use salvo::prelude::*;

#[handler]
async fn index(res: &mut Response) {
    res.render(Text::Html(r#"
        <!DOCTYPE html>
        <html>
        <body>
            <h1>Upload File</h1>
            <form action="/" method="post" enctype="multipart/form-data">
                <input type="file" name="file" />
                <input type="submit" value="Upload" />
            </form>
        </body>
        </html>
    "#));
}

#[handler]
async fn upload(req: &mut Request, res: &mut Response) {
    let file = req.file("file").await;

    if let Some(file) = file {
        let dest = format!("temp/{}", file.name().unwrap_or("file"));
        println!("Uploading to: {}", dest);

        if let Err(e) = std::fs::copy(file.path(), Path::new(&dest)) {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Text::Plain(format!("Upload failed: {e}")));
        } else {
            res.render(Text::Plain(format!("File uploaded to {dest}")));
        }
    } else {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Text::Plain("No file in request"));
    }
}

#[tokio::main]
async fn main() {
    std::fs::create_dir_all("temp").unwrap();

    let router = Router::new().get(index).post(upload);

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## Multiple File Upload

```rust
use std::path::Path;
use salvo::prelude::*;

#[handler]
async fn upload_files(req: &mut Request, res: &mut Response) {
    let files = req.files("files").await;

    if let Some(files) = files {
        let mut uploaded = Vec::with_capacity(files.len());

        for file in files {
            let dest = format!("temp/{}", file.name().unwrap_or("file"));

            if let Err(e) = std::fs::copy(file.path(), Path::new(&dest)) {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                res.render(Text::Plain(format!("Failed to upload: {e}")));
                return;
            } else {
                uploaded.push(dest);
            }
        }

        res.render(Text::Plain(format!(
            "Files uploaded:\n\n{}",
            uploaded.join("\n")
        )));
    } else {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Text::Plain("No files in request"));
    }
}
```

## File Validation

```rust
#[handler]
async fn upload_image(req: &mut Request, res: &mut Response) {
    let file = req.file("image").await;

    let Some(file) = file else {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render("No file provided");
        return;
    };

    // Validate content type
    let content_type = file.content_type().unwrap_or_default();
    let allowed_types = ["image/jpeg", "image/png", "image/gif", "image/webp"];

    if !allowed_types.contains(&content_type) {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(format!(
            "Invalid file type: {}. Allowed: {}",
            content_type,
            allowed_types.join(", ")
        ));
        return;
    }

    // Validate file size (5MB max)
    let max_size = 5 * 1024 * 1024;
    if file.size() > max_size {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(format!(
            "File too large: {} bytes. Max: {} bytes",
            file.size(),
            max_size
        ));
        return;
    }

    // Generate unique filename
    let filename = file.name().unwrap_or("unnamed");
    let extension = filename.rsplit('.').next().unwrap_or("");
    let unique_name = format!(
        "{}_{}.{}",
        uuid::Uuid::new_v4(),
        chrono::Utc::now().timestamp(),
        extension
    );
    let dest = format!("uploads/{}", unique_name);

    if let Err(e) = std::fs::copy(file.path(), &dest) {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        res.render(format!("Upload failed: {e}"));
    } else {
        res.render(Json(serde_json::json!({
            "filename": unique_name,
            "original_name": filename,
            "size": file.size(),
            "content_type": content_type
        })));
    }
}
```

## Size Limiting

```rust
use salvo::prelude::*;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .push(
            Router::with_path("upload")
                .hoop(max_size(10 * 1024 * 1024))  // 10MB limit
                .post(upload_handler)
        );

    let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
    Server::new(acceptor).serve(router).await;
}
```

## File Download

### Basic Download

```rust
use salvo::fs::NamedFile;
use salvo::prelude::*;

#[handler]
async fn download(req: &mut Request, res: &mut Response) {
    let filename: String = req.param("filename").unwrap();
    let filepath = format!("files/{}", filename);

    match NamedFile::builder(&filepath)
        .attached_name(&filename)
        .send(req.headers(), res)
        .await
    {
        Ok(_) => {}
        Err(_) => {
            res.status_code(StatusCode::NOT_FOUND);
            res.render("File not found");
        }
    }
}
```

### Inline View (PDF, Images)

```rust
#[handler]
async fn view_pdf(req: &mut Request, res: &mut Response) {
    match NamedFile::builder("documents/report.pdf")
        .content_type("application/pdf")
        .send(req.headers(), res)
        .await
    {
        Ok(_) => {}
        Err(_) => {
            res.status_code(StatusCode::NOT_FOUND);
        }
    }
}
```

### Protected Downloads

```rust
#[handler]
async fn protected_download(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) {
    let user = depot.get::<User>("user");
    if user.is_none() {
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render("Please login");
        return;
    }

    let filename: String = req.param("filename").unwrap();

    // Validate filename to prevent directory traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render("Invalid filename");
        return;
    }

    let filepath = format!("private/{}", filename);

    match NamedFile::builder(&filepath)
        .attached_name(&filename)
        .send(req.headers(), res)
        .await
    {
        Ok(_) => {}
        Err(_) => {
            res.status_code(StatusCode::NOT_FOUND);
            res.render("File not found");
        }
    }
}
```

## Form with File and Other Fields

```rust
use salvo::prelude::*;

#[handler]
async fn update_profile(req: &mut Request, res: &mut Response) {
    // Parse form fields
    let name = req.form::<String>("name").await.unwrap_or_default();
    let bio = req.form::<String>("bio").await.unwrap_or_default();

    // Handle file upload
    let avatar_path = if let Some(file) = req.file("avatar").await {
        let filename = format!("avatar_{}.jpg", uuid::Uuid::new_v4());
        let dest = format!("uploads/avatars/{}", filename);
        std::fs::copy(file.path(), &dest).ok();
        Some(filename)
    } else {
        None
    };

    res.render(Json(serde_json::json!({
        "name": name,
        "bio": bio,
        "avatar": avatar_path
    })));
}
```

## Best Practices

1. Always validate file types before processing
2. Limit file sizes to prevent resource exhaustion
3. Generate unique filenames to avoid collisions
4. Validate filenames to prevent directory traversal
5. Use separate directories for uploads
6. Consider using streaming for large files
7. Clean up temporary files periodically
8. Set appropriate permissions on upload directories

## Related Skills

- **salvo-static-files**: Serve uploaded files
- **salvo-openapi**: Document file upload endpoints
