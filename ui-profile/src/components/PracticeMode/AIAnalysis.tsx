import { useState, useEffect, FC } from 'react';

interface AIAnalysisProps {
  answer: string;
  onInsights: (insights: string[]) => void;
}

export const AIAnalysis: FC<AIAnalysisProps> = ({ answer, onInsights }) => {
  const [insights, setInsights] = useState<string[]>([]);

  useEffect(() => {
    if (answer.length < 30) {
      setInsights([]);
      onInsights([]);
      return;
    }

    // Debounced analysis
    const timer = setTimeout(() => {
      const newInsights: string[] = [];

      // Check for STAR structure
      const hasSituation = answer.match(/I (was|had|needed|decided|joined|started)/i);
      const hasTask = answer.match(/I (needed to|had to|was tasked with|decided to)/i);
      const hasAction = answer.match(/I (did|created|built|implemented|led|organized)/i);
      const hasResult = answer.match(/As a result|Result|Outcome|Therefore|So|Led to/i);

      if (!hasSituation) {
        newInsights.push('💡 Consider starting with the Situation ("I was...")');
      }
      
      if (!hasTask) {
        newInsights.push('💡 Clarify your Task or responsibility in the situation');
      }
      
      if (!hasAction) {
        newInsights.push('💡 Describe specific Actions you took (use "I" statements)');
      }
      
      if (!hasResult) {
        newInsights.push('💡 End with the Result or outcome of your actions');
      }

      // Check for "I" statements (not "we")
      const weCount = (answer.match(/\bwe\b/gi) || []).length;
      const iCount = (answer.match(/\bI\b/gi) || []).length;
      
      if (weCount > 0 && iCount === 0) {
        newInsights.push('💡 Use "I" instead of "we" to highlight your personal contribution');
      } else if (weCount > iCount * 2) {
        newInsights.push('💡 Balance "we" with more "I" statements to show individual impact');
      }

      // Check for metrics/results
      const hasMetrics = answer.match(/\d+%|\d+x|\$\d+|\d+(\.\d+)?\s*(ms|s|seconds?|minutes?|hours?|days?|weeks?|months?|years?)/i);
      
      if (!hasMetrics) {
        newInsights.push('💡 Add metrics or quantifiable results when possible (%, $, time saved, etc.)');
      }

      // Check for specificity
      const vagueWords = answer.match(/\b(some|many|several|few|various|different|things|stuff)\b/gi);
      if (vagueWords && vagueWords.length > 2) {
        newInsights.push('💡 Replace vague terms with specific examples and details');
      }

      // Check for learning/reflection
      const hasLearning = answer.match(/\b(learned|realized|discovered|found out|now know|would do differently)\b/i);
      if (!hasLearning && answer.length > 100) {
        newInsights.push('💡 Consider what you learned from this experience');
      }

      setInsights(newInsights);
      onInsights(newInsights);
    }, 1500);

    return () => clearTimeout(timer);
  }, [answer]);

  return null; // The component doesn't render anything directly, insights are passed via callback
};