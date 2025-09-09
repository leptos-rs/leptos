# Server Function Boilerplate

**Issue ID**: LEPTOS-2024-004  
**Severity**: Medium  
**Category**: Developer Experience  
**Status**: Open  
**Created**: 2024-09-08  
**Updated**: 2024-09-08  

## Problem Statement

### Current State
Server functions require repetitive boilerplate for common patterns like database access, error handling, and context extraction. This leads to verbose code and potential inconsistencies across applications.

### Impact Assessment
- **Developer Impact**: 🟡 **Medium** - Repetitive patterns, verbose code, potential errors
- **Adoption Impact**: 🟡 **Medium** - Makes server functions appear more complex than necessary  
- **Maintenance Impact**: 🟡 **Medium** - Code duplication across projects
- **Performance Impact**: ⚪ **None** - No performance impact

### Evidence

**Current Repetitive Patterns**:
```rust
#[server]
pub async fn get_posts() -> Result<Vec<Post>, ServerFnError> {
    use sqlx::PgPool;
    
    // Repeated in every server function
    let pool = use_context::<PgPool>()
        .expect("Database pool should be provided");
    
    // Repeated error handling pattern
    let posts = sqlx::query_as!(
        Post,
        "SELECT id, title, content, created_at FROM posts ORDER BY created_at DESC"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    
    Ok(posts)
}

#[server]  
pub async fn create_post(title: String, content: String) -> Result<Post, ServerFnError> {
    use sqlx::PgPool;
    
    // Same boilerplate repeated
    let pool = use_context::<PgPool>()
        .expect("Database pool should be provided");
    
    // Same error handling pattern
    let post = sqlx::query_as!(
        Post,
        "INSERT INTO posts (title, content) VALUES ($1, $2) RETURNING *",
        title,
        content
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    
    Ok(post)
}

#[server]
pub async fn get_user() -> Result<User, ServerFnError> {
    use sqlx::PgPool;
    
    // Repeated context extraction
    let pool = use_context::<PgPool>()
        .expect("Database pool should be provided");
    
    // Repeated auth pattern  
    let session = use_context::<Session>()
        .ok_or_else(|| ServerFnError::ServerError("Not authenticated".to_string()))?;
    
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = $1",
        session.user_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    
    Ok(user)
}
```

**Community Feedback**:
- "Server functions are very verbose for simple database operations"
- "Lots of copy-paste between server functions"
- "Error handling is inconsistent across my server functions"
- "Wish there was a way to reduce the boilerplate"

## Root Cause Analysis

### Technical Analysis
- No built-in helpers for common patterns
- Context extraction requires manual boilerplate
- Error conversion is manual and repetitive
- No conventions for database access patterns

### Design Analysis
- Framework provides low-level primitives without high-level conveniences
- No opinionated patterns for common use cases
- Missing abstraction layer for database operations
- Error handling left entirely to user implementation

## Proposed Solution

### Overview
Provide optional helper macros and utility functions for common server function patterns while maintaining the flexibility of the current system.

### Technical Approach

**1. Database-Aware Server Functions**
```rust
// Proposed: Database helper attributes
#[server]
#[database] // Auto-injects database pool
pub async fn get_posts(db: &Database) -> Result<Vec<Post>, ServerFnError> {
    let posts = db.query_as!(
        Post,
        "SELECT id, title, content, created_at FROM posts ORDER BY created_at DESC"
    ).fetch_all().await?; // Auto error conversion
    
    Ok(posts)
}

// Or with table helpers
#[server]
#[table("posts")]
pub async fn get_posts() -> Result<Vec<Post>, ServerFnError> {
    // Implementation auto-generated for basic queries
    Post::find_all().await
}
```

**2. Context Injection Helpers**
```rust
// Proposed: Automatic context injection
#[server]
pub async fn get_user(
    #[context] db: PgPool,
    #[context] session: Session,
) -> Result<User, ServerFnError> {
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = $1",
        session.user_id
    )
    .fetch_one(&db)
    .await?; // Auto error conversion
    
    Ok(user)
}
```

**3. Error Handling Conveniences**
```rust
// Proposed: Automatic error conversion
#[server]
#[error_convert] // Automatically converts common errors
pub async fn create_post(title: String, content: String) -> Result<Post, ServerFnError> {
    let db = use_db(); // Helper function
    
    let post = sqlx::query_as!(
        Post,
        "INSERT INTO posts (title, content) VALUES ($1, $2) RETURNING *",
        title,
        content
    )
    .fetch_one(&db)
    .await?; // No manual error conversion needed
    
    Ok(post)
}
```

**4. Query Builder Integration**
```rust
// Proposed: Optional query builder integration
#[server]
pub async fn get_posts_by_author(author_id: i32) -> Result<Vec<Post>, ServerFnError> {
    use leptos_db::prelude::*;
    
    let posts = Post::table()
        .filter(Post::author_id.eq(author_id))
        .order_by(Post::created_at.desc())
        .load()
        .await?;
    
    Ok(posts)
}
```

### Alternative Approaches
1. **Built-in ORM**: Create full ORM integration - Too opinionated
2. **Macro-heavy**: Generate everything automatically - Reduces flexibility  
3. **Trait-based**: Use traits for common patterns - More complex API

## Implementation Plan

### Phase 1: Foundation (3 weeks)
- [ ] Design helper attribute system
- [ ] Create context injection mechanisms
- [ ] Implement error conversion helpers
- [ ] Basic database helper functions

### Phase 2: Implementation (4 weeks)
- [ ] Database attribute implementation
- [ ] Context injection attributes
- [ ] Error handling improvements
- [ ] Query builder integration

### Phase 3: Polish (2 weeks)
- [ ] Documentation and examples
- [ ] Integration testing
- [ ] Performance validation
- [ ] Community feedback incorporation

### Success Criteria
- 70% reduction in server function boilerplate for common patterns
- Maintained performance characteristics
- Optional adoption (doesn't break existing code)
- Clear migration path from verbose patterns

## Risk Assessment

### Implementation Risks
- **Complexity**: Additional macro complexity in codebase
- **Magic**: Too much "magic" could reduce code clarity
- **Flexibility**: Helpers might not cover all use cases

**Mitigation Strategies**:
- Keep helpers optional and composable
- Maintain escape hatches for complex cases
- Clear documentation of what helpers do
- Community feedback during development

### Breaking Changes
✅ **No Breaking Changes** - Purely additive features

## Testing Strategy

### Unit Tests
- Helper attribute functionality
- Context injection correctness
- Error conversion accuracy

### Integration Tests
- Real database interactions
- Complex server function patterns
- Performance impact measurement

### Performance Tests
- Macro compilation time impact
- Runtime performance comparison
- Memory usage validation

### User Acceptance Tests
- Boilerplate reduction measurement
- Code clarity improvements
- Learning curve impact

## Documentation Requirements

### API Documentation
- Helper attribute reference
- Context injection guide
- Error handling patterns

### User Guides
- "Server Function Best Practices" guide
- Migration from verbose patterns
- Database integration patterns

### Migration Guides
- Gradual adoption guide (optional migration)

## Community Impact

### Backward Compatibility
✅ **Full Compatibility** - All existing code works unchanged

### Learning Curve
📈 **Moderate Improvement** - Reduces verbose patterns, clearer examples

### Ecosystem Impact
🎯 **Positive** - Encourages consistent patterns across community

---

**Related Issues**: None  
**Dependencies**: None  
**Assignee**: TBD  
**Milestone**: v0.9.0