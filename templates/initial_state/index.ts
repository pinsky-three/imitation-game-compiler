import { BrowserContext, Page, Stagehand } from "@browserbasehq/stagehand";
import boxen from "boxen";
import chalk from "chalk";
import StagehandConfig from "./stagehand.config.js";

/**
 * 🤘 Welcome to Stagehand! Thanks so much for trying us out!
 * 🛠️ CONFIGURATION: stagehand.config.ts will help you configure Stagehand
 *
 * 📝 Check out our docs for more fun use cases, like building agents
 * https://docs.stagehand.dev/
 *
 * 💬 If you have any feedback, reach out to us on Slack!
 * https://stagehand.dev/slack
 *
 * 📚 You might also benefit from the docs for Zod, Browserbase, and Playwright:
 * - https://zod.dev/
 * - https://docs.browserbase.com/
 * - https://playwright.dev/docs/intro
 */
async function main({
  page,
  context,
  stagehand,
}: {
  page: Page; // Playwright Page with act, extract, and observe methods
  context: BrowserContext; // Playwright BrowserContext
  stagehand: Stagehand; // Stagehand instance
}) {
  // Navigate to the starting URL (placeholder)
  await page.goto("__START_URL__");

  // --- Generated Action Sequence Start ---
  // __ACTION_SEQUENCE__
  // --- Generated Action Sequence End ---

  // Optional: Add final checks or logging if needed after the sequence
  stagehand.log({
    category: "generated-script",
    message: `Finished executing generated action sequence.`,
  });

  // Example: Log final metrics
  // stagehand.log({
  //   category: "generated-script-metrics",
  //   message: `Metrics`,
  //   auxiliary: {
  //     metrics: {
  //       value: JSON.stringify(stagehand.metrics),
  //       type: "object",
  //     },
  //   },
  // });
}

/**
 * This is the main function that runs when you do npm run start
 *
 * YOU PROBABLY DON'T NEED TO MODIFY ANYTHING BELOW THIS POINT!
 *
 */
async function run() {
  const stagehand = new Stagehand({
    ...StagehandConfig,
  });
  await stagehand.init();

  if (StagehandConfig.env === "BROWSERBASE" && stagehand.browserbaseSessionID) {
    console.log(
      boxen(
        `View this session live in your browser: \n${chalk.blue(
          `https://browserbase.com/sessions/${stagehand.browserbaseSessionID}`
        )}`,
        {
          title: "Browserbase",
          padding: 1,
          margin: 3,
        }
      )
    );
  }

  const page = stagehand.page;
  const context = stagehand.context;
  await main({
    page,
    context,
    stagehand,
  });
  await stagehand.close();
  console.log(
    `\n🤘 Thanks so much for using Stagehand! Reach out to us on Slack if you have any feedback: ${chalk.blue(
      "https://stagehand.dev/slack"
    )}\n`
  );
}

run();
