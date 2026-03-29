const { chromium } = require("playwright")
const path = require("path")

;(async () => {
  const browser = await chromium.launch()
  const context = await browser.newContext({
    viewport: { width: 1200, height: 630 },
    deviceScaleFactor: 2
  })

  // OG image (1200x630)
  const ogPage = await context.newPage()
  await ogPage.goto("file://" + path.resolve(__dirname, "og-image.html"))
  await ogPage.screenshot({
    path: path.resolve(__dirname, "..", "og-image.png")
  })
  console.log("Created og-image.png (1200x630 @2x)")

  // Favicon PNGs from SVG
  const sizes = [32, 180, 192, 512]
  for (const size of sizes) {
    const page = await context.newPage()
    await page.setViewportSize({ width: size, height: size })
    await page.goto("file://" + path.resolve(__dirname, "favicon.svg"))
    await page.screenshot({
      path: path.resolve(__dirname, "..", `favicon-${size}.png`)
    })
    console.log(`Created favicon-${size}.png`)
    await page.close()
  }

  await browser.close()
  console.log("Done.")
})()
