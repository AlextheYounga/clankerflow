# Workflow Examples

These examples are intentionally short. They are meant to show how `clankerflow` workflows read at a glance, 
not every capability of the runtime. Some are intentionally absurd, because why not. 

>NOTE: Most of these were AI-generated.

## Planner -> Dev -> QA

```ts
async function showcase(ctx, { agent, tickets, git }) {
  // Let the planner create a small, realistic unit of work.
  await agent.run({ title: "Planner", prompt: "Create one small feature ticket." });

  // Use a caller-provided ticket when present; otherwise grab the next open one.
  const ticket = ctx.ticket ?? (await tickets.getNext({ status: "OPEN" })).ticket;
  if (!ticket) throw new Error("No open ticket found");

  // Work happens on the ticket branch so each run stays isolated and reviewable.
  await git.checkoutBranch(ticket.branch ?? `ticket-${ticket.ticketId}`, "master");

  // Move the ticket into active work before handing it to the implementation agent.
  await tickets.updateStatus({ id: ticket.ticketId, status: "IN_PROGRESS" });

  await agent.run({ title: "Dev", prompt: `Implement ${ticket.filePath}` });
  await agent.run({ title: "QA", prompt: `Review ${ticket.filePath}` });
}
```

## Run A Specific Ticket

```ts
async function buildTicket(ctx, { agent, git }) {
  if (!ctx.ticket) throw new Error("This workflow requires a ticket");

  await git.checkoutBranch(
    ctx.ticket.branch ?? `ticket-${ctx.ticket.ticketId}`,
    "master"
  );

  await agent.run({
    title: `Build ${ctx.ticket.ticketId}`,
    prompt: `Implement ${ctx.ticket.filePath}`,
  });
}
```

## Daily Release Notes

```ts
async function releaseNotes(_ctx, { agent, git }) {
  const commits = await git.log(["--oneline", "-5"]);

  await agent.run({
    title: "Release Notes",
    prompt: `Turn these recent commits into crisp release notes:\n\n${commits.stdout}`,
  });
}
```

## Process The Next Open Ticket

```ts
async function nextUp(_ctx, { agent, tickets }) {
  const { ticket } = await tickets.getNext({ status: "OPEN" });
  if (!ticket) return { ok: true, skipped: true };

  await tickets.updateStatus({ id: ticket.ticketId, status: "IN_PROGRESS" });
  await agent.run({ title: ticket.title, prompt: `Implement ${ticket.filePath}` });
}
```

## Add A Ticket Note

```ts
async function documentResult(ctx, { tickets }) {
  if (!ctx.ticket) throw new Error("This workflow requires a ticket");

  await tickets.comment({
    id: ctx.ticket.ticketId,
    text: "Implementation is ready for review.",
    section: "Dev Notes",
  });
}
```

## Start An Agent Session And Follow Up

```ts
async function reviewFlow(_ctx, { agent }) {
  const result = await agent.run({
    title: "Implement feature",
    prompt: "Build the requested change and summarize what you changed.",
  });

  if (!result.session_id) return result;

  await agent.command({
    session_id: result.session_id,
    command: "/review",
  });

  return result;
}
```

## Unstick A Blocked Ticket

```ts
async function unblock(_ctx, { agent, tickets }) {
  const { tickets: all } = await tickets.list();
  const stuck = all.find((ticket) => ticket.status === "STUCK");
  if (!stuck) return { ok: true, skipped: true };

  await agent.run({
    title: `Unblock ${stuck.ticketId}`,
    prompt: `Read ${stuck.filePath} and propose the smallest path forward.`,
  });
}
```

## Debate Before You Build

```ts
async function debate(_ctx, { agent, fs }) {
  const brief = await fs.read("docs/PROJECT.md");

  await agent.run({
    title: "Optimist",
    prompt: `Read this project brief and make the best case for building it now:\n\n${brief}`,
  });

  await agent.run({
    title: "Skeptic",
    prompt: `Read the same brief and argue against it using risks, scope, and edge cases:\n\n${brief}`,
  });
}
```

## Friday Mode

```ts
async function friday(ctx, { agent, git, tickets }) {
  const ticket = ctx.ticket ?? (await tickets.getNext({ status: "OPEN" })).ticket;
  if (!ticket) throw new Error("No open ticket found");

  await git.checkoutBranch(ticket.branch ?? `ticket-${ticket.ticketId}`, "master");

  await agent.run({
    title: ctx.yolo ? "Careful Cowboy" : "Steady Engineer",
    prompt: ctx.yolo
      ? `You are in yolo mode. Make the smallest safe change possible in ${ticket.filePath}.`
      : `Implement ${ticket.filePath} conservatively and explain each step.`,
  });
}
```

## Tiny Lawyer

```ts
async function tinyLawyer(_ctx, { agent, fs }) {
  const readme = await fs.read("README.md");

  await agent.run({
    title: "Writer",
    prompt: `Improve this README intro without changing the product meaning:\n\n${readme}`,
  });

  await agent.run({
    title: "Tiny Lawyer",
    prompt: `Now review that README draft and object to anything vague, risky, or misleading.`,
  });
}
```

## Hype Squad
```ts
async function hypeSquad(_ctx, { agent, git }) {
  const diff = await git.diff();

  await agent.run({
    title: "Hype Squad",
    prompt: `Read this diff and announce it like the most important release of the decade:\n\n${diff.stdout}`,
  });
}
```

## Existential Review
```ts
async function existentialReview(_ctx, { agent, fs }) {
  const brief = await fs.read("docs/PROJECT.md");

  await agent.run({
    title: "Philosopher",
    prompt: `Should this project exist at all? Argue from first principles:\n\n${brief}`,
  });

  await agent.run({
    title: "Pragmatist",
    prompt: `Ignore philosophy and decide whether this project is worth building this month:\n\n${brief}`,
  });
}
```

## Sim City

```ts
async function simCity(_ctx, { agent, fs }) {
  let city = await fs.read("docs/PROJECT.md");

  for (let day = 1; day <= 3; day++) {
    await agent.run({
      title: `Mayor Day ${day}`,
      prompt: `Given this city plan, propose one ambitious improvement:\n\n${city}`,
    });

    await agent.run({
      title: `Budget Office Day ${day}`,
      prompt: "Now cut one expensive idea and force a compromise.",
    });

    await agent.run({
      title: `Citizen Day ${day}`,
      prompt: "Now complain loudly about whatever the other two decided.",
    });
  }
}
```

## Refactor Roulette

```ts
async function refactorRoulette(_ctx, { agent, git }) {
  const diff = await git.diff();

  for (const role of ["Minimalist", "Performance Nerd", "Future Maintainer"]) {
    await agent.run({
      title: role,
      prompt: `Review this diff and suggest one refactor from your perspective:\n\n${diff.stdout}`,
    });
  }
}
```

## Committee Of One Too Many

```ts
async function committee(_ctx, { agent, fs }) {
  const readme = await fs.read("README.md");

  for (const role of ["Designer", "Engineer", "PM", "Pedant"]) {
    await agent.run({
      title: role,
      prompt: `Review this README and leave one opinionated note:\n\n${readme}`,
    });
  }
}
```


## Idea Tournament

```ts
async function ideaTournament(_ctx, { agent, fs }) {
  const brief = await fs.read("docs/PROJECT.md");

  let champion = "the current approach";
  for (const challenger of ["CLI-first", "container-first", "tickets-first"]) {
    await agent.run({
      title: `${challenger} vs ${champion}`,
      prompt: `Given this brief, argue why ${challenger} should beat ${champion}:\n\n${brief}`,
    });
    champion = challenger;
  }
}
```
