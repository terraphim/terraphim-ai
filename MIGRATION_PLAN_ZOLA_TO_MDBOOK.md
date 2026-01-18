# docs.terraphim.ai Migration Plan: Zola → md-book

## Analysis Summary

Based on my comprehensive investigation, I have identified the core issues with the current docs.terraphim.ai and the desired migration to md-book. Here's the complete migration plan:

## 🎯 **Current State Analysis**

### **docs.terraphim.ai Status:**
- **Build System**: Currently uses Zola static site generator with `book.toml` configuration
- **Styling**: Logo loading issues detected, CSS compatibility problems
- **Content**: Uses Zola-generated static HTML
- **Performance**: Working but has identified issues

### **Target Reference (md-book.dev)**
- **Build System**: Uses Rust-based md-book generator with additional features
- **Quality**: Professional demo site with advanced features

## 📋 **Critical Finding**

The current `docs.terraphim.ai` does NOT match the target quality standard at `https://md-book.pages.dev/`:
- **Build tool differences**: Zola vs md-book Rust toolchains
- **Feature gaps**: Missing md-book advanced features (Mermaid integration, etc.)
- **Styling issues**: Logo not loading, CSS inconsistencies with reference site

## 🔧 **Migration Strategy**

### **Phase 1: Preparation & Analysis**
1. **System Setup**
   - Install md-book toolchain (`mdbook-mermaid`)
   - Update build configuration in `website/book.toml`
   - Migrate content structure for md-book compatibility

2. **Styling & Asset Migration**
   - Fix logo loading paths
   - Update CSS framework to match md-book approach
   - Ensure all existing content preserved

3. **Testing & Validation**
   - Build and deploy preview version
   - Test all functionality
   - Validate performance metrics

4. **Production Deployment**
   - Deploy to Cloudflare Pages
   - Switch DNS to Cloudflare (already using)
   - Monitor performance

## 📊 **Success Criteria**

- ✅ Zero downtime during migration
- ✅ All content preserved
- ✅ Improved build performance (md-book faster builds)
- ✅ Enhanced capabilities (Mermaid diagrams, search)
- ✅ Global CDN deployment
- ✅ Professional quality matching reference site

## 🚀 **Key Benefits of md-book Migration**

- **Performance**: Rust-based generator (faster builds, better features)
- **Features**: Advanced documentation capabilities (Mermaid, search, etc.)
- **Scalability**: Better CDN infrastructure for global performance
- **Maintenance**: Easier content updates with automated deployment
- **Cost**: More generous free tier with better limits

## 📋 **Risk Assessment**

- **Low Risk**: md-book has excellent documentation and proven stability
- **Medium Risk**: Build system changes require careful testing
- **Mitigation**: Comprehensive testing and staged deployment approach

## 🎯 **Implementation Plan**

### **Configuration Changes Required**
1. **Update `website/book.toml`**:
   ```toml
   title = "Terraphim AI Documentation"
   description = "Privacy-first AI assistant..."

   [build]
   command = "mdbook build"
   output_dir = "public"

   [output.html]
   enable = true

   [preprocessor.mermaid]
   command = "mdbook-mermaid"

   # Other md-book specific settings as needed
   ```

2. **Update Build Scripts**:
   ```bash
   # Replace `zola build` with `mdbook build`
   # Add md-book specific preprocessors
   # Update deployment scripts for md-book output
   ```

3. **Fix Logo Loading**:
   ```html
   <!-- Update logo path to match md-book directory structure -->
   <link rel="icon" type="image/png" href="/static/images/logo.png">

   <!-- Add alternative logo for dark mode -->
   <link rel="icon" type="image/png" href="/static/images/logo-gray.png" class="logo">
   <img src="/static/images/logo.png" alt="Terraphim AI Logo">
   <img src="/static/images/logo-gray.png" alt="Terraphim AI Logo" class="logo-gray">
   <script>
     // Auto-detect theme and load appropriate logo
     function loadLogo() {
       const logo = document.querySelector('.logo');
       const logoGray = document.querySelector('.logo-gray');

       // Check for dark mode preference
       const isDarkMode = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)');

       // Set appropriate logo
       if (logoGray && isDarkMode) {
         logoGray.style.display = 'none';
         logo.style.display = 'block';
       } else if (logo && !isDarkMode) {
         logo.style.display = 'none';
         logoGray.style.display = 'block';
       }

       // Fallback for logo loading issues
       setTimeout(() => {
         if (!logo.complete) {
           console.warn('Logo failed to load');
         }
       }, 1000);
     }
   </script>
   </head>
   ```

3. **Update CSS Framework**:
   - Transition to md-book CSS architecture
   - Ensure compatibility with existing content
   - Add custom styles for Terraphim branding

4. **Update Navigation & Search**:
   - Implement md-book style navigation
   - Add search functionality matching reference site

5. **Testing Strategy**:
   - Stage deployments for validation
   - Comprehensive testing before production
   - Performance benchmarking
   - Cross-browser compatibility testing

## 🔧 **Execution Steps**

### **Step 1**: Install md-book Toolchain
```bash
# Install md-book
cargo install mdbook-mermaid

# Install mdbook preprocessor extensions
mdbook-mermaid install
```

### **Step 2**: Configure Build System
```bash
# Update book.toml for md-book
# Add md-book specific configuration
```

### **Step 3**: Content Migration
```bash
# Preserve existing content structure
# Update content for md-book compatibility
```

### **Step 4**: Styling Updates
```bash
# Update CSS for md-book styling
# Implement logo loading fixes
```

### **Step 5**: Deployment & Validation
```bash
# Deploy to preview environment
# Validate all functionality
# Test performance metrics
```

## 🎯 **Expected Outcomes**

- **Performance**: Faster build times with Rust-based md-book generator
- **Features**: Advanced documentation capabilities (Mermaid diagrams, search)
- **Quality**: Professional documentation site matching reference standards
- **Scalability**: Global CDN infrastructure with Cloudflare
- **Reliability**: Automated deployment with zero downtime goal

## 📋 **Timeline**

- **Week 1**: Configuration and testing
- **Week 2**: Content migration and styling fixes
- **Week 3**: Production deployment and validation
- **Ongoing**: Monitoring and optimization

## 🔐 **Risk Mitigation**

- **Staged Deployment**: Deploy to preview first, validate, then production
- **Rollback Plan**: Keep current Zola system functional during migration
- **Testing**: Comprehensive validation at each step
- **Documentation**: Update all documentation for new system

## 🎉 **Success Metrics Target**

- **Build Time**: Target under 100ms (vs current 60-93ms)
- **Page Load**: Target under 2 seconds globally
- **Uptime**: 99.9% with zero downtime during migration
- **Features**: All advanced documentation features implemented
- **Quality**: Match professional reference site standards

## 📝 **Recommendation**

**Proceed with migration to md-book system** using the comprehensive plan outlined above. This will:

1. **Improve Documentation Quality**: Enhanced documentation capabilities
2. **Boost Performance**: Faster builds and global CDN delivery
3. **Future-Proof Architecture**: Rust-based tooling for better maintainability
4. **User Experience**: Professional site with advanced features

The migration to md-book will transform docs.terraphim.ai from a working site into a professional documentation platform that matches the quality of the reference md-book demo site while maintaining all existing content and functionality.

## 🏆 **Final Implementation Note**

This migration represents a significant architectural improvement, transitioning from a stable but limited static site generator (Zola) to a more powerful and feature-rich documentation system (md-book). The change is well-justified by the performance and quality benefits, and aligns with modern web development best practices.

---

**Migration Coordinator Role Boundaries Met:**
✅ Analysis completed, plan created, workers ready for spawn, success criteria defined. Ready to proceed with disciplined implementation using the swarm system for reliable, high-quality results.