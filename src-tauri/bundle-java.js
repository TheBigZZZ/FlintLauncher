#!/usr/bin/env node

/**
 * Tauri build script to pre-download and bundle Java runtimes
 * Downloads Java from Mojang servers and includes in the installer
 * 
 * This runs as part of the Tauri build process:
 * - Downloads Java components to src-tauri/resources/java-runtime/
 * - Java files are bundled into the MSI/EXE installer
 * - On first app launch, Java is copied to %APPDATA%/.flint/runtime/
 */

import fs from 'fs';
import path from 'path';
import https from 'https';
import zlib from 'zlib';
import { pipeline } from 'stream';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const JAVA_MANIFEST_URL = 'https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json';
const RESOURCES_DIR = path.join(__dirname, 'resources', 'java-runtime');

const COMPONENTS_TO_BUNDLE = [
    'jre-legacy',           // For older Minecraft versions
    'java-runtime-alpha',   // For newer versions (Java 16)
    'java-runtime-gamma',   // Alternative Java runtime
];

// Parallel download configuration
const MAX_PARALLEL_DOWNLOADS = 32;

// Queue for managing parallel downloads
class DownloadQueue {
    constructor(maxConcurrent) {
        this.maxConcurrent = maxConcurrent;
        this.activeDownloads = 0;
        this.queue = [];
    }

    async add(downloadFn) {
        return new Promise((resolve, reject) => {
            this.queue.push({ downloadFn, resolve, reject });
            this.processQueue();
        });
    }

    async processQueue() {
        while (this.activeDownloads < this.maxConcurrent && this.queue.length > 0) {
            this.activeDownloads++;
            const { downloadFn, resolve, reject } = this.queue.shift();
            
            try {
                const result = await downloadFn();
                resolve(result);
            } catch (err) {
                reject(err);
            } finally {
                this.activeDownloads--;
                this.processQueue();
            }
        }
    }
}

function ensureDir(dir) {
    if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
    }
}

function downloadJson(url) {
    return new Promise((resolve, reject) => {
        https.get(url, (res) => {
            let data = '';
            res.on('data', chunk => data += chunk);
            res.on('end', () => {
                try {
                    resolve(JSON.parse(data));
                } catch (e) {
                    reject(e);
                }
            });
        }).on('error', reject);
    });
}

function downloadFile(url, filepath) {
    return new Promise((resolve, reject) => {
        const file = fs.createWriteStream(filepath);
        
        https.get(url, (res) => {
            // Handle redirects
            if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
                file.destroy();
                return downloadFile(res.headers.location, filepath).then(resolve).catch(reject);
            }
            
            if (res.statusCode !== 200) {
                file.destroy();
                fs.unlink(filepath, () => {});
                reject(new Error(`HTTP ${res.statusCode}: ${url}`));
                return;
            }

            // Handle compression based on content-encoding header
            let stream = res;
            if (res.headers['content-encoding'] === 'gzip') {
                stream = res.pipe(zlib.createGunzip());
            }

            stream.pipe(file);
            
            file.on('finish', () => {
                file.close();
                resolve();
            });

            file.on('error', (err) => {
                fs.unlink(filepath, () => {});
                reject(err);
            });

            stream.on('error', (err) => {
                fs.unlink(filepath, () => {});
                reject(err);
            });
        }).on('error', (err) => {
            file.destroy();
            fs.unlink(filepath, () => {});
            reject(err);
        });
    });
}

async function downloadJavaComponent(component) {
    console.log(`\n📥 Downloading ${component}...`);
    
    try {
        // Fetch manifest
        const allRuntimes = await downloadJson(JAVA_MANIFEST_URL);
        const runtimeList = allRuntimes['windows-x64'][component];
        
        if (!runtimeList || runtimeList.length === 0) {
            console.warn(`⚠️  No ${component} found for windows-x64`);
            return;
        }

        // Get latest version
        const runtime = runtimeList[runtimeList.length - 1];
        const manifestUrl = runtime.manifest.url;
        const componentDir = path.join(RESOURCES_DIR, component);
        
        ensureDir(componentDir);
        
        // Fetch file manifest
        console.log(`   Fetching manifest from ${manifestUrl}`);
        const fileManifest = await downloadJson(manifestUrl);
        const files = fileManifest.files || {};
        
        const fileEntries = Object.entries(files);
        const totalFiles = fileEntries.filter(([_, info]) => info.type === 'file').length;
        
        console.log(`   Found ${totalFiles} files to download (${MAX_PARALLEL_DOWNLOADS} parallel threads)`);
        
        // Create directories first
        for (const [filePath, fileInfo] of fileEntries) {
            if (fileInfo.type === 'directory') {
                const dirPath = path.join(componentDir, filePath.replace(/\//g, path.sep));
                ensureDir(dirPath);
            }
        }

        // Prepare file downloads
        const downloadQueue = new DownloadQueue(MAX_PARALLEL_DOWNLOADS);
        let downloadedCount = 0;
        let errorCount = 0;
        
        const downloadTasks = fileEntries
            .filter(([_, info]) => info.type === 'file')
            .map(([filePath, fileInfo]) => {
                return async () => {
                    const fileUrl = fileInfo.downloads?.raw?.url;
                    if (!fileUrl) return;

                    const fullPath = path.join(componentDir, filePath.replace(/\//g, path.sep));

                    // Skip if already exists
                    if (fs.existsSync(fullPath)) {
                        downloadedCount++;
                        if (downloadedCount % 50 === 0) {
                            process.stdout.write(`\r   ${downloadedCount}/${totalFiles} files (${MAX_PARALLEL_DOWNLOADS} threads)`);
                        }
                        return;
                    }

                    try {
                        await downloadFile(fileUrl, fullPath);
                        downloadedCount++;
                        if (downloadedCount % 50 === 0) {
                            process.stdout.write(`\r   ${downloadedCount}/${totalFiles} files (${MAX_PARALLEL_DOWNLOADS} threads)`);
                        }
                    } catch (err) {
                        errorCount++;
                        // Log but continue - one failed file shouldn't stop everything
                        if (errorCount <= 5) {  // Limit error output
                            console.error(`\n   ⚠️  Failed to download ${filePath}: ${err.message}`);
                        }
                    }
                };
            });

        // Execute all downloads with parallel queue
        await Promise.all(downloadTasks.map(task => downloadQueue.add(task)));
        
        console.log(`\r   ✅ ${component} downloaded (${downloadedCount}/${totalFiles} files)`);
        if (errorCount > 0) {
            console.warn(`   ⚠️  ${errorCount} files failed to download (will retry on next build)`);
        }
        
    } catch (err) {
        console.error(`❌ Failed to download ${component}: ${err.message}`);
        throw err;
    }
}

async function main() {
    console.log(`\n${'='.repeat(60)}`);
    console.log('☕ Flint Launcher - Java Runtime Bundling');
    console.log(`${'='.repeat(60)}`);
    console.log(`\nBundling location: ${RESOURCES_DIR}`);
    console.log(`Components to bundle: ${COMPONENTS_TO_BUNDLE.join(', ')}`);
    console.log(`Parallel downloads: ${MAX_PARALLEL_DOWNLOADS} threads`);
    console.log('\nThis may take 2-5 minutes depending on internet speed...\n');

    ensureDir(RESOURCES_DIR);

    // Check if bundling is disabled
    const skipBundleFile = path.join(__dirname, '.skip-java-bundle');
    if (fs.existsSync(skipBundleFile)) {
        console.log('⏭️  Java bundling disabled (create an empty file named .skip-java-bundle to disable)');
        return;
    }

    let successCount = 0;
    
    for (const component of COMPONENTS_TO_BUNDLE) {
        try {
            await downloadJavaComponent(component);
            successCount++;
        } catch (err) {
            console.warn(`⚠️  Skipping ${component} - will be downloaded on first app launch`);
        }
    }

    console.log(`\n${'='.repeat(60)}`);
    if (successCount > 0) {
        console.log(`✅ Java bundling complete! (${successCount}/${COMPONENTS_TO_BUNDLE.length} components)`);
        console.log('   These files will be included in the MSI installer');
        console.log('   On first app launch, Java will be extracted to:');
        console.log('   %APPDATA%/.flint/runtime/');
    } else {
        console.log('⚠️  No Java components were bundled');
        console.log('   Java will be downloaded on first app launch');
    }
    console.log(`${'='.repeat(60)}\n`);
}

main().catch(err => {
    console.error(`\n❌ Fatal error: ${err.message}`);
    // Don't fail the build - Java downloads on demand if bundling fails
    process.exit(0);
});

