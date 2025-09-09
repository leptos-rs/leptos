# Leptos Init Examples

This document provides practical examples of using the `leptos init` command for different types of projects.

## 🏪 E-commerce Website

Create a full-featured e-commerce site with database and styling:

```bash
leptos init ecommerce \
  --template fullstack \
  --server axum \
  --database postgresql \
  --styling tailwind \
  --tracing \
  --islands
```

**Generated Features:**
- Server-side rendering with hydration
- PostgreSQL database integration
- Tailwind CSS styling
- Structured logging
- Islands architecture for performance
- Complete project structure

**Next Steps:**
```bash
cd ecommerce
cargo leptos watch
# Add your e-commerce logic
```

## 📝 Personal Blog

Create a static blog with modern styling:

```bash
leptos init blog \
  --template static \
  --styling tailwind
```

**Generated Features:**
- Static site generation
- Tailwind CSS for styling
- SEO-optimized structure
- Fast loading times
- CDN-friendly output

**Next Steps:**
```bash
cd blog
cargo leptos watch
# Add your blog content and styling
```

## 🔧 API Service

Create a backend API with database:

```bash
leptos init api-service \
  --template api \
  --server axum \
  --database sqlite \
  --tracing
```

**Generated Features:**
- RESTful API endpoints
- SQLite database integration
- Structured logging
- No frontend dependencies
- Optimized for server deployment

**Next Steps:**
```bash
cd api-service
cargo leptos watch
# Add your API endpoints
```

## 📊 Admin Dashboard

Create a client-side admin dashboard:

```bash
leptos init admin-dashboard \
  --template spa \
  --styling tailwind \
  --islands
```

**Generated Features:**
- Client-side rendering
- Tailwind CSS styling
- Islands architecture
- No server dependencies
- Fast initial load

**Next Steps:**
```bash
cd admin-dashboard
cargo leptos watch
# Build your dashboard components
```

## 🎮 Game Frontend

Create a game frontend with minimal dependencies:

```bash
leptos init game-frontend \
  --template spa
```

**Generated Features:**
- Minimal client-side setup
- No styling framework (add your own)
- Optimized for performance
- Canvas/WebGL ready

**Next Steps:**
```bash
cd game-frontend
cargo leptos watch
# Add your game logic
```

## 📱 Mobile App Backend

Create a backend for mobile applications:

```bash
leptos init mobile-backend \
  --template api \
  --server axum \
  --database postgresql \
  --tracing
```

**Generated Features:**
- RESTful API for mobile apps
- PostgreSQL for data persistence
- Structured logging for monitoring
- CORS-ready configuration

**Next Steps:**
```bash
cd mobile-backend
cargo leptos watch
# Add mobile API endpoints
```

## 🏢 Enterprise Application

Create a complex enterprise application:

```bash
leptos init enterprise-app \
  --template custom
```

**Interactive Setup:**
```
📋 Select project template:
1. Fullstack (SSR + Client hydration) - Recommended
2. SPA (Client-side only)
3. Static (Static site generation)
4. API (Server functions only)
5. Custom (Interactive configuration)

Enter choice (1-5): 1

Server backend:
1. Axum (Recommended)
2. Actix Web
3. Warp

Select backend (1-3): 1

Add database support? (y/n): y

Database:
1. SQLite (Recommended)
2. PostgreSQL
3. MySQL

Select database (1-3): 2

Add Tailwind CSS? (y/n): y

Enable tracing? (y/n): y

Enable islands architecture? (y/n): y

🚀 Create project with this configuration? (y/n): y
```

## 🔄 Migration from Existing Project

If you have an existing Leptos project, you can use the generated structure as a reference:

```bash
# Create a new project with similar configuration
leptos init migrated-app --template fullstack --database postgresql

# Compare the generated Cargo.toml with your existing one
# Copy over your custom dependencies and features
# Update your source files to match the new structure
```

## 🎨 Custom Styling Examples

### With Tailwind CSS
```bash
leptos init styled-app --template fullstack --styling tailwind
```

**Generated Tailwind Setup:**
```css
/* src/styles/tailwind.css */
@tailwind base;
@tailwind components;
@tailwind utilities;

.navbar {
    @apply flex space-x-4 p-4 bg-gray-100;
}

.container {
    @apply max-w-4xl mx-auto p-4;
}
```

### With Vanilla CSS
```bash
leptos init vanilla-app --template fullstack --styling vanilla-css
```

**Add your custom styles:**
```css
/* src/styles/main.css */
.navbar {
    display: flex;
    gap: 1rem;
    padding: 1rem;
    background-color: #f3f4f6;
}

.container {
    max-width: 64rem;
    margin: 0 auto;
    padding: 1rem;
}
```

### With Sass
```bash
leptos init sass-app --template fullstack --styling sass
```

**Use Sass features:**
```scss
// src/styles/main.scss
$primary-color: #3b82f6;
$secondary-color: #6b7280;

.navbar {
    display: flex;
    gap: 1rem;
    padding: 1rem;
    background-color: $primary-color;
    
    &:hover {
        background-color: darken($primary-color, 10%);
    }
}
```

## 🚀 Deployment Examples

### Static Hosting (Vercel, Netlify)
```bash
leptos init static-site --template static --styling tailwind
```

**Deploy:**
```bash
cd static-site
cargo leptos build --release
# Deploy the target/site directory
```

### Docker Deployment
```bash
leptos init docker-app --template fullstack --database postgresql
```

**Dockerfile:**
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo leptos build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/site /app
WORKDIR /app
EXPOSE 3000
CMD ["./server"]
```

### Cloud Functions
```bash
leptos init cloud-api --template api --server axum
```

**Deploy to AWS Lambda, Google Cloud Functions, or Azure Functions**

## 🔍 Debugging Examples

### Enable Tracing
```bash
leptos init debug-app --template fullstack --tracing
```

**Use in your code:**
```rust
use tracing::{info, warn, error};

#[component]
pub fn MyComponent() -> impl IntoView {
    info!("Component rendered");
    
    view! {
        <div>"Hello World"</div>
    }
}
```

### Development vs Production
```bash
# Development setup
leptos init dev-app --template fullstack --database sqlite

# Production setup
leptos init prod-app --template fullstack --database postgresql --tracing
```

## 📚 Learning Examples

### Beginner Project
```bash
leptos init learn-leptos --template spa
```

**Simple counter example:**
```rust
use leptos::*;

#[component]
pub fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    
    view! {
        <div>
            <button on:click=move |_| set_count.update(|n| *n -= 1)>"-"</button>
            <span>{count}</span>
            <button on:click=move |_| set_count.update(|n| *n += 1)>"+"</button>
        </div>
    }
}
```

### Intermediate Project
```bash
leptos init todo-app --template fullstack --database sqlite
```

**Add server functions:**
```rust
#[server]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    // Database query logic
    Ok(vec![])
}

#[server]
pub async fn add_todo(title: String) -> Result<(), ServerFnError> {
    // Database insert logic
    Ok(())
}
```

### Advanced Project
```bash
leptos init advanced-app --template fullstack --database postgresql --styling tailwind --islands
```

**Use islands architecture:**
```rust
#[component]
pub fn InteractiveWidget() -> impl IntoView {
    // This component will be rendered as an island
    // Only the interactive parts are hydrated
    view! {
        <div class="widget">
            <button>"Click me"</button>
        </div>
    }
}
```

---

**Ready to build something amazing?** Start with `leptos init my-project` and let the magic begin! 🚀
