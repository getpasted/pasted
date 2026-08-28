import fs from 'node:fs';
import { pathToFileURL } from 'node:url';

const stableReleaseTag = /^v\d+\.\d+\.\d+$/;

const argumentValue = (argumentsList, name) => {
  const index = argumentsList.indexOf(name);
  if (index < 0 || !argumentsList[index + 1]) throw new Error(`Missing ${name}`);
  return argumentsList[index + 1];
};

export const validateAnnouncement = ({ repository, tag, title, body }) => {
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
    throw new Error(`Invalid repository: ${repository}`);
  }
  if (!stableReleaseTag.test(tag)) throw new Error(`Announcements require a stable release tag: ${tag}`);
  if (!title.trim() || title.length > 120) throw new Error('Announcement title must contain 1–120 characters.');

  const expectedMarker = `<!-- pasted-release:${tag} -->`;
  const markers = body.match(/<!-- pasted-release:[^>]+ -->/g) ?? [];
  if (markers.length !== 1 || markers[0] !== expectedMarker) {
    throw new Error(`Announcement must contain exactly one ${expectedMarker} marker.`);
  }
  if (!body.trim() || body.length > 65_536) {
    throw new Error('Announcement body must contain 1–65,536 characters.');
  }
  return expectedMarker;
};

export const findExistingAnnouncement = (discussions, marker) => {
  const matches = discussions.filter(({ body }) => body.includes(marker));
  if (matches.length > 1) throw new Error(`Multiple Discussions contain ${marker}.`);
  return matches[0] ?? null;
};

const requestGraphql = async (token, query, variables) => {
  const response = await fetch('https://api.github.com/graphql', {
    method: 'POST',
    headers: {
      Accept: 'application/vnd.github+json',
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
      'User-Agent': 'pasted-release-announcement',
    },
    body: JSON.stringify({ query, variables }),
  });
  const result = await response.json();
  if (!response.ok || result.errors?.length) {
    throw new Error(`GitHub GraphQL request failed: ${JSON.stringify(result.errors ?? result)}`);
  }
  return result.data;
};

const publish = async (argumentsList = process.argv.slice(2)) => {
  const repository = argumentValue(argumentsList, '--repository');
  const tag = argumentValue(argumentsList, '--tag');
  const title = argumentValue(argumentsList, '--title');
  const bodyFile = argumentValue(argumentsList, '--body-file');
  const token = process.env.GH_TOKEN;
  if (!token) throw new Error('Missing GH_TOKEN.');

  const body = fs.readFileSync(bodyFile, 'utf8');
  const marker = validateAnnouncement({ repository, tag, title, body });
  const [owner, name] = repository.split('/');
  const repositoryData = await requestGraphql(
    token,
    `query($owner: String!, $name: String!) {
      repository(owner: $owner, name: $name) {
        id
        discussionCategories(first: 50) { nodes { id name slug } }
        discussions(first: 100, orderBy: { field: CREATED_AT, direction: DESC }) {
          nodes { id number title url body }
        }
      }
    }`,
    { owner, name },
  );
  const target = repositoryData.repository;
  if (!target) throw new Error(`Repository not found: ${repository}`);

  const existing = findExistingAnnouncement(target.discussions.nodes, marker);
  if (existing) {
    console.log(`Release announcement already exists: ${existing.url}`);
    return existing;
  }

  const category = target.discussionCategories.nodes.find(({ slug }) => slug === 'announcements');
  if (!category) throw new Error('The repository has no Announcements Discussion category.');

  const created = await requestGraphql(
    token,
    `mutation($repositoryId: ID!, $categoryId: ID!, $title: String!, $body: String!) {
      createDiscussion(input: {
        repositoryId: $repositoryId,
        categoryId: $categoryId,
        title: $title,
        body: $body
      }) { discussion { id number title url } }
    }`,
    { repositoryId: target.id, categoryId: category.id, title, body },
  );
  console.log(`Published release announcement: ${created.createDiscussion.discussion.url}`);
  return created.createDiscussion.discussion;
};

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  publish().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}

