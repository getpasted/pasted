import type {
  RegisteredContentType,
  RegisteredContentTypeGroup,
} from './ContentTypeProvider';
import type { SmartBinFeatures } from './binModalModel';

export interface UseBinModalTargetsInput {
  contentTypes: RegisteredContentType[];
  contentTypeGroups: RegisteredContentTypeGroup[];
  features: SmartBinFeatures;
  fileFormats: string[];
  sources: string[];
  installedApps: string[];
}
