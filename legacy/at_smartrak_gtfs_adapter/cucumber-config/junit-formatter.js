let cucumber = require("@cucumber/cucumber");

class JUnitFormatter extends cucumber.Formatter {
    constructor(options) {
        super(options);
        
        this.eventDataCollector = options.eventDataCollector;
        options.eventBroadcaster.on("envelope", (envelope) => {
            if (envelope.testCaseFinished) {
                this.onTestCaseFinished(envelope.testCaseFinished);
            } else if (envelope.testRunFinished) {
                this.onTestRunFinished();
            }
        });
        this.failures = 0;
        this.skipped = 0;
        this.tests = 0;
        this.time = 0;
    }
    onTestCaseFinished(testCaseFinished) {
        const testCaseAttempt = this.eventDataCollector.getTestCaseAttempt(testCaseFinished.testCaseStartedId)
        const parsed = cucumber.formatterHelpers.parseTestCaseAttempt({
            cwd: this.cwd,
            snippetBuilder: this.snippetBuilder,
            supportCodeLibrary: this.supportCodeLibrary,
            testCaseAttempt
        });
        let failed = false;
        let skipped = false;
        let duration = 0;
        parsed.testSteps.forEach(testStep => {
            duration += testStep.result.duration.nanos;
            if (testStep.result.status == "FAILED") {
                failed = true;
            } else if (testStep.result.status == "SKIPPED") {
                skipped = true;
            }
        });
        this.tests++;
        this.time += duration;
        if (failed) {
            this.failures++;
        }
        if (skipped) {
            this.skipped++;
        }
    }
    parseString(str) {
        return str.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/'/g, "&apos;");
    }
    onTestRunFinished() {
        this.log(`<?xml version="1.0" encoding="UTF-8" standalone="no"?>\n`);
        this.log(`<testsuite failures="${this.failures}" name="JUnit-Formatter" skipped="${this.skipped}" tests="${this.tests}" time="${Math.round(this.time / 1000000)}">\n`);
        const testCaseAttempts = this.eventDataCollector.getTestCaseAttempts();
        for (let testCaseAttempt of testCaseAttempts) {
            this.formatTestCaseAttempt(testCaseAttempt);
        }
        this.log(`</testsuite>`);
    }
    formatTestCaseAttempt(testCaseAttempt) {
        const feature = testCaseAttempt.gherkinDocument.feature;
        const messages = [];
        let duration = 0;
        for (let testStep of testCaseAttempt.testCase.testSteps) {
            const stepResult = testCaseAttempt.stepResults[testStep.id];
            duration += stepResult.duration.nanos;
            if (testStep.pickleStepId) {
                const stepInstance = testCaseAttempt.pickle.steps.find((s) => s.id === testStep.pickleStepId);
                const step = this.findStepByScenario(feature, testCaseAttempt.pickle.astNodeIds[0], stepInstance.astNodeIds[0]);
                if (step) {
                    if (step.argument && step.argument.content) {
                        messages.push(`(${this.parseString(stepResult.status)}) ${this.parseString(step.keyword)}${this.parseString(stepInstance.text)} \n ${this.parseString(step.argument.content)}\n`);
                    } else {
                        messages.push(`(${this.parseString(stepResult.status)}) ${this.parseString(step.keyword)}${this.parseString(stepInstance.text)} \n`);
                    }
                }
            }
        }
        this.log(`<testcase classname="${this.parseString(feature.name)}" owner="${this.parseString(feature.name)}" name="${this.parseString(testCaseAttempt.pickle.name)}" time="${Math.round(duration / 1000000)}">\n`);
        this.log(`<system-out><![CDATA[`);
        for (let message in messages) {
            this.log(messages[message]);
        }
        this.log(`]]></system-out>\n`);
        this.log(`</testcase>\n`);
    }
    findStepByScenario(feature, scenarioId, stepId) {
        const scenario = feature.children.find((child) => child.scenario && child.scenario.id === scenarioId);
        if (scenario) {
            return scenario.scenario.steps.find((step) => step.id === stepId);
        }
        return null;
    }
}

exports.default = JUnitFormatter;
