# Deploying to GitHub Pages

## ✨ Automatic Deployment (Recommended)

The project uses GitHub Actions for automatic deployment!

**Just push to main:**
```bash
git add .
git commit -m "Your changes"
git push origin main
```

GitHub Actions will automatically:
1. ✅ Build the WASM module
2. ✅ Build the web app
3. ✅ Deploy to GitHub Pages

Check the **Actions** tab in your GitHub repo to see deployment progress.

Your site will be live at: **https://friendlymatthew.github.io/mycelium/**

## 🏠 Local Development

For local development, you don't need to deploy:

```bash
cd web
npm run dev
```

## 🔧 Manual Deployment (If Needed)

If GitHub Actions isn't working or you need to deploy manually:

```bash
./deploy-gh-pages.sh
cd web/dist
git add .
git commit -m "Deploy"
git push -f origin main:gh-pages
```

## ⚙️ GitHub Pages Setup

Make sure GitHub Pages is configured:
1. Go to **Settings** → **Pages**
2. Source: `gh-pages` branch, `/ (root)` folder
3. Save

## 🔄 Updating Data

When you add new GPS data:

```bash
# Export new data
./export-data.sh

# Commit and push (automatic deployment will handle the rest)
git add web/public/gps-data.json
git commit -m "Update GPS data"
git push origin main
```

## 🐛 Troubleshooting

**Deployment failed?**
- Check the Actions tab for error logs
- Make sure all dependencies are in package.json
- Verify wasm-pack builds locally first

**Site not updating?**
- Clear your browser cache
- Check the gh-pages branch was updated
- Wait a few minutes for GitHub to rebuild
